//! Streamed sound on the web: decoded one packet at a time as it plays.
//!
//! Kira streams on a decoder thread of its own, which a browser page
//! doesn't have, so on the web a streamed sound keeps its encoded bytes and
//! [`ChunkedSound`] decodes the next packet whenever the mixer has used up
//! the last one, on the thread that mixes (the page's). Only one packet's
//! samples are held decoded at a time instead of the whole track.
//!
//! It is compiled for the web, and natively for its tests.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering::Relaxed};

use kira::info::Info;
use kira::sound::{PlaybackState, Sound, SoundData};
use kira::{Frame, Panning, interpolate_frame};
use symphonia::core::codecs::{CodecParameters, audio::AudioDecoder};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::{FormatReader, TrackType, probe::Hint};
use symphonia::core::io::MediaSourceStream;

/// How long a stop fades out: Kira's default tween, which the other voices use.
const STOP_FADE_SECONDS: f64 = 0.01;

/// How quickly volume and panning follow a change, in seconds (about one
/// mixer block), so a change doesn't click.
const SMOOTHING_SECONDS: f64 = 0.01;

/// A sound's header: its sample rate and, when the file says, its length.
pub(crate) struct Header {
    pub sample_rate: u32,
    pub frames: Option<u64>,
}

impl Header {
    /// Length in seconds; 0 when the file doesn't say.
    pub fn seconds(&self) -> f32 {
        self.frames
            .map_or(0.0, |frames| (frames as f64 / f64::from(self.sample_rate)) as f32)
    }
}

/// One decoder over the encoded bytes: the container reader and the codec.
struct Decoding {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track: u32,
}

fn open(bytes: &Arc<[u8]>) -> Result<(Decoding, Header), String> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.clone())), Default::default());
    let reader = symphonia::default::get_probe()
        .probe(&Hint::default(), source, Default::default(), Default::default())
        .map_err(|e| e.to_string())?;
    let track = reader.default_track(TrackType::Audio).ok_or("it has no audio track")?;
    let Some(CodecParameters::Audio(params)) = track.codec_params.as_ref() else {
        return Err("it has no audio track".into());
    };
    let sample_rate = params
        .sample_rate
        .filter(|&rate| rate > 0)
        .ok_or("its sample rate is unknown")?;
    let header = Header {
        sample_rate,
        frames: track.num_frames,
    };
    let decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &Default::default())
        .map_err(|e| e.to_string())?;
    let track = track.id;
    Ok((Decoding { reader, decoder, track }, header))
}

/// Reads the header of encoded `bytes`, decoding no samples.
pub(crate) fn header(bytes: &Arc<[u8]>) -> Result<Header, String> {
    open(bytes).map(|(_, header)| header)
}

/// What a voice's handle and its sound share.
struct Shared {
    /// Linear gain, as `f32` bits.
    volume: AtomicU32,
    /// -1 (left) to 1 (right), as `f32` bits.
    panning: AtomicU32,
    /// Playback speed, as `f64` bits.
    rate: AtomicU64,
    stop: AtomicBool,
    /// 0 playing, 1 stopping (fading out), 2 stopped.
    state: AtomicU8,
}

const PLAYING: u8 = 0;
const STOPPING: u8 = 1;
const STOPPED: u8 = 2;

/// A streamed sound to play: the encoded bytes and how to play them.
pub(crate) struct ChunkedData {
    bytes: Arc<[u8]>,
    volume: f32,
    rate: f64,
    looping: bool,
}

impl ChunkedData {
    /// Plays `bytes` at linear gain `volume` and speed `rate`, looping or not.
    pub fn new(bytes: Arc<[u8]>, volume: f32, rate: f64, looping: bool) -> Self {
        Self {
            bytes,
            volume: gain(volume),
            rate: speed(rate),
            looping,
        }
    }
}

fn gain(volume: f32) -> f32 {
    if volume.is_finite() { volume.max(0.0) } else { 0.0 }
}

fn speed(rate: f64) -> f64 {
    if rate.is_finite() { rate.max(0.01) } else { 1.0 }
}

impl SoundData for ChunkedData {
    type Error = String;
    type Handle = ChunkedHandle;

    fn into_sound(self) -> Result<(Box<dyn Sound>, ChunkedHandle), String> {
        let (sound, handle) = ChunkedSound::new(self)?;
        Ok((Box::new(sound), handle))
    }
}

/// Controls a playing [`ChunkedSound`], like Kira's own sound handles.
pub(crate) struct ChunkedHandle {
    shared: Arc<Shared>,
}

impl ChunkedHandle {
    pub fn state(&self) -> PlaybackState {
        match self.shared.state.load(Relaxed) {
            PLAYING => PlaybackState::Playing,
            STOPPING => PlaybackState::Stopping,
            _ => PlaybackState::Stopped,
        }
    }

    /// Fades out over 10 ms, then stops.
    pub fn stop(&mut self) {
        self.shared.stop.store(true, Relaxed);
    }

    /// Linear gain, smoothed.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub fn set_volume(&mut self, volume: f32) {
        self.shared.volume.store(gain(volume).to_bits(), Relaxed);
    }

    /// -1 (left) to 1 (right), smoothed.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub fn set_panning(&mut self, panning: f32) {
        let panning = if panning.is_finite() {
            panning.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.shared.panning.store(panning.to_bits(), Relaxed);
    }

    /// Playback speed (which also changes the pitch).
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub fn set_playback_rate(&mut self, rate: f64) {
        self.shared.rate.store(speed(rate).to_bits(), Relaxed);
    }
}

/// A playing streamed sound: decodes a packet whenever it has played the
/// last one, and resamples to the output rate.
pub(crate) struct ChunkedSound {
    bytes: Arc<[u8]>,
    /// `None` once the end is reached (and it doesn't loop) or decoding failed.
    decoding: Option<Decoding>,
    sample_rate: u32,
    looping: bool,
    /// The current packet's samples, interleaved, reused for every packet.
    samples: Vec<f32>,
    channels: usize,
    /// The next frame of `samples` to play.
    next: usize,
    /// The frames the resampler interpolates between: previous, current
    /// and the next two.
    window: [Frame; 4],
    /// How far playback is from `window[1]` towards `window[2]`.
    fraction: f64,
    /// Frames of silence pushed into the window after the end; once the
    /// current frame is one of them (3), the sound is over.
    tail: u8,
    volume: f32,
    panning: f32,
    /// 1, falling to 0 while it stops.
    fade: f64,
    shared: Arc<Shared>,
}

impl ChunkedSound {
    fn new(data: ChunkedData) -> Result<(Self, ChunkedHandle), String> {
        let (decoding, header) = open(&data.bytes)?;
        let shared = Arc::new(Shared {
            volume: AtomicU32::new(data.volume.to_bits()),
            panning: AtomicU32::new(0f32.to_bits()),
            rate: AtomicU64::new(data.rate.to_bits()),
            stop: AtomicBool::new(false),
            state: AtomicU8::new(PLAYING),
        });
        let mut sound = Self {
            bytes: data.bytes,
            decoding: Some(decoding),
            sample_rate: header.sample_rate,
            looping: data.looping,
            samples: Vec::new(),
            channels: 1,
            next: 0,
            window: [Frame::ZERO; 4],
            fraction: 0.0,
            tail: 0,
            volume: data.volume,
            panning: 0.0,
            fade: 1.0,
            shared: shared.clone(),
        };
        for _ in 1..4 {
            sound.advance();
        }
        Ok((sound, ChunkedHandle { shared }))
    }

    /// Decoded samples held right now, in bytes: one packet's worth.
    #[cfg(test)]
    fn decoded_bytes(&self) -> usize {
        self.samples.capacity() * std::mem::size_of::<f32>()
    }

    fn done(&self) -> bool {
        self.tail >= 3 || self.fade <= 0.0
    }

    /// Moves the window on by one source frame.
    fn advance(&mut self) {
        let next = match self.next_frame() {
            Some(frame) => frame,
            None => {
                self.tail = self.tail.saturating_add(1);
                Frame::ZERO
            }
        };
        self.window = [self.window[1], self.window[2], self.window[3], next];
    }

    /// The next source frame, decoding a packet when the last one is used
    /// up; `None` at the end.
    fn next_frame(&mut self) -> Option<Frame> {
        let mut reopened = false;
        while self.next * self.channels >= self.samples.len() {
            let decoding = self.decoding.as_mut()?;
            match decoding.reader.next_packet() {
                Ok(Some(packet)) if packet.track_id == decoding.track => match decoding.decoder.decode(&packet) {
                    Ok(buffer) => {
                        self.channels = buffer.num_planes().max(1);
                        buffer.copy_to_vec_interleaved(&mut self.samples);
                        self.next = 0;
                        reopened = false;
                    }
                    // A damaged packet: skip it, as players do.
                    Err(DecodeError::DecodeError(_)) => {}
                    Err(_) => self.end(&mut reopened),
                },
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => self.end(&mut reopened),
            }
        }
        let at = self.next * self.channels;
        self.next += 1;
        let left = self.samples[at];
        Some(if self.channels == 1 {
            Frame::from_mono(left)
        } else {
            Frame::new(left, self.samples[at + 1])
        })
    }

    /// The decoder reached the end: start over when looping (once per
    /// frame asked for, so a sound without samples can't loop forever),
    /// else stop decoding.
    fn end(&mut self, reopened: &mut bool) {
        self.decoding = if self.looping && !*reopened {
            *reopened = true;
            open(&self.bytes).ok().map(|(decoding, _)| decoding)
        } else {
            None
        };
    }
}

impl Sound for ChunkedSound {
    fn on_start_processing(&mut self) {
        if self.shared.stop.load(Relaxed) && self.shared.state.load(Relaxed) == PLAYING {
            self.shared.state.store(STOPPING, Relaxed);
        }
    }

    fn process(&mut self, out: &mut [Frame], dt: f64, _info: &Info) {
        let volume = f32::from_bits(self.shared.volume.load(Relaxed));
        let panning = f32::from_bits(self.shared.panning.load(Relaxed));
        let step = f64::from_bits(self.shared.rate.load(Relaxed)) * f64::from(self.sample_rate) * dt;
        let smoothing = (dt / SMOOTHING_SECONDS).min(1.0) as f32;
        let stopping = self.shared.stop.load(Relaxed);
        for frame in out {
            if self.done() {
                *frame = Frame::ZERO;
                continue;
            }
            self.volume += (volume - self.volume) * smoothing;
            self.panning += (panning - self.panning) * smoothing;
            if stopping {
                self.fade -= dt / STOP_FADE_SECONDS;
            }
            let [previous, current, next_1, next_2] = self.window;
            let sample = interpolate_frame(previous, current, next_1, next_2, self.fraction as f32);
            *frame = (sample * self.volume * self.fade.max(0.0) as f32).panned(Panning(self.panning));
            self.fraction += step;
            while self.fraction >= 1.0 && !self.done() {
                self.fraction -= 1.0;
                self.advance();
            }
        }
        if self.done() {
            self.decoding = None;
            self.shared.state.store(STOPPED, Relaxed);
        }
    }

    fn finished(&self) -> bool {
        self.done()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit WAV of `samples` at `rate` Hz.
    fn wav(rate: u32, samples: &[i16]) -> Arc<[u8]> {
        let data = samples.len() as u32 * 2;
        let mut out = Vec::from(&b"RIFF"[..]);
        out.extend_from_slice(&(36 + data).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&[1, 0, 1, 0]); // PCM, mono
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * 2).to_le_bytes());
        out.extend_from_slice(&[2, 0, 16, 0]);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data.to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out.into()
    }

    /// A ramp that never repeats within `n` samples, so frames are told apart.
    fn ramp(n: usize) -> Vec<i16> {
        (0..n).map(|i| ((i % 20_000) as i16) + 1).collect()
    }

    fn play(bytes: &Arc<[u8]>, looping: bool) -> (ChunkedSound, ChunkedHandle) {
        ChunkedSound::new(ChunkedData::new(bytes.clone(), 1.0, 1.0, looping)).unwrap()
    }

    fn run(sound: &mut ChunkedSound, frames: usize, dt: f64) -> Vec<Frame> {
        let info = kira::info::MockInfoBuilder::new().build();
        let mut out = vec![Frame::ZERO; frames];
        // In blocks, as the mixer asks.
        for block in out.chunks_mut(128) {
            sound.on_start_processing();
            sound.process(block, dt, &info);
        }
        out
    }

    #[test]
    fn chunked_stream_plays_the_samples_whole_decoding_gives() {
        let bytes = wav(8000, &ramp(5000));
        let whole = kira::sound::static_sound::StaticSoundData::from_cursor(Cursor::new(bytes.clone())).unwrap();
        let (mut sound, _handle) = play(&bytes, false);
        let out = run(&mut sound, 5000, 1.0 / 8000.0);
        assert_eq!(out.len(), whole.frames.len());
        for (i, (got, want)) in out.iter().zip(whole.frames.iter()).enumerate() {
            assert!((got.left - want.left).abs() < 1e-6, "frame {i}: {got:?} != {want:?}");
        }
    }

    #[test]
    fn chunked_stream_holds_one_packet_not_the_whole_track() {
        // A minute at 8 kHz: 480 000 frames, 3.84 MB decoded as Kira frames.
        let bytes = wav(8000, &ramp(480_000));
        let whole_bytes = 480_000 * std::mem::size_of::<Frame>();
        let (mut sound, _handle) = play(&bytes, false);
        run(&mut sound, 8000 * 10, 1.0 / 8000.0);
        assert!(!sound.finished());
        assert!(sound.decoded_bytes() > 0);
        assert!(
            sound.decoded_bytes() * 100 < whole_bytes,
            "{} bytes decoded at once",
            sound.decoded_bytes()
        );
    }

    #[test]
    fn chunked_stream_finishes_at_the_end_unless_it_loops() {
        let bytes = wav(8000, &ramp(100));
        let (mut once, handle) = play(&bytes, false);
        run(&mut once, 300, 1.0 / 8000.0);
        assert!(once.finished());
        assert_eq!(handle.state(), PlaybackState::Stopped);

        let (mut looping, handle) = play(&bytes, true);
        let out = run(&mut looping, 1000, 1.0 / 8000.0);
        assert!(!looping.finished());
        assert_eq!(handle.state(), PlaybackState::Playing);
        assert_eq!(out[0].left, out[100].left);
        assert_eq!(out[37].left, out[937].left);
    }

    #[test]
    fn chunked_stream_resamples_to_the_output_rate() {
        let bytes = wav(8000, &ramp(800));
        let (mut sound, _handle) = play(&bytes, false);
        // A 16 kHz output plays each source frame twice as long.
        let out = run(&mut sound, 1590, 1.0 / 16_000.0);
        assert!(!sound.finished());
        assert!((out[400].left - ramp(800)[200] as f32 / 32768.0).abs() < 1e-6);
        run(&mut sound, 20, 1.0 / 16_000.0);
        assert!(sound.finished());
    }

    #[test]
    fn stopping_a_chunked_stream_fades_out_then_ends() {
        let bytes = wav(8000, &ramp(8000));
        let (mut sound, mut handle) = play(&bytes, true);
        run(&mut sound, 128, 1.0 / 8000.0);
        handle.stop();
        // 5 ms into the 10 ms fade.
        let out = run(&mut sound, 40, 1.0 / 8000.0);
        assert_eq!(handle.state(), PlaybackState::Stopping);
        assert!(!sound.finished());
        assert!(out[39].left.abs() < out[0].left.abs());
        let out = run(&mut sound, 128, 1.0 / 8000.0);
        assert!(sound.finished());
        assert_eq!(handle.state(), PlaybackState::Stopped);
        assert_eq!(out[127], Frame::ZERO);
    }

    #[test]
    fn chunked_stream_follows_volume_changes_smoothly() {
        let bytes = wav(8000, &[16_384; 4000]);
        let (mut sound, mut handle) = play(&bytes, false);
        handle.set_volume(0.0);
        let out = run(&mut sound, 400, 1.0 / 8000.0);
        // No jump to silence on the first frame; under 1% within 50 ms.
        assert!(out[0].left > 0.4);
        assert!(out[399].left.abs() < out[0].left * 0.01);
    }

    #[test]
    fn chunked_stream_follows_panning_and_speed() {
        let bytes = wav(8000, &[16_384; 8000]);
        let (mut sound, mut handle) = play(&bytes, false);
        handle.set_panning(-1.0);
        let out = run(&mut sound, 400, 1.0 / 8000.0);
        assert!(out[399].right.abs() < out[399].left.abs() * 0.1, "{:?}", out[399]);
        // Twice as fast: the 7600 frames left take 3800.
        handle.set_playback_rate(2.0);
        run(&mut sound, 3700, 1.0 / 8000.0);
        assert!(!sound.finished());
        run(&mut sound, 200, 1.0 / 8000.0);
        assert!(sound.finished());
    }

    #[test]
    fn chunked_header_reads_the_length_and_rejects_garbage() {
        let header = header(&wav(8000, &ramp(4000))).unwrap();
        assert_eq!(header.sample_rate, 8000);
        assert!((header.seconds() - 0.5).abs() < 1e-6);
        assert!(super::header(&Arc::from(&b"not a sound"[..])).is_err());
        assert!(ChunkedSound::new(ChunkedData::new(Arc::from(&b""[..]), 1.0, 1.0, true)).is_err());
    }
}
