//! Sound effects and music, played with `kira`.
//!
//! The audio device opens the first time a sound is loaded or played. If
//! there is none (servers, CI), or the `audio` feature is off, everything
//! here still works and is simply silent, so game code never needs a
//! special case.

use crate::ctx::Ctx;

/// A loaded sound. Cheap to copy. Get one from
/// [`Ctx::load_sound`](crate::Ctx::load_sound) or
/// [`Ctx::create_sound`](crate::Ctx::create_sound).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Sound {
    id: u32,
    /// Length in seconds.
    seconds: f32,
}

impl Sound {
    /// Length in seconds.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(sound: Sound) {
    /// println!("{:.1} s", sound.seconds());
    /// # }
    /// ```
    pub fn seconds(&self) -> f32 {
        self.seconds
    }
}

/// A sound that was started, to stop it later with
/// [`Ctx::stop_sound`](crate::Ctx::stop_sound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Playing(u64);

/// Optional settings for [`Ctx::play_sound_ex`](crate::Ctx::play_sound_ex).
///
/// ```
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx, music: Sound) {
/// ctx.play_sound_ex(&music, SoundParams { volume: 0.5, looping: true, ..Default::default() });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundParams {
    /// 0 is silent, 1 (default) is as recorded.
    pub volume: f32,
    /// Playback speed, which also changes the pitch: 2 is twice as fast
    /// and an octave higher. Default 1.
    pub speed: f32,
    /// Start over at the end, for music and ambience. Default `false`.
    pub looping: bool,
}

impl Default for SoundParams {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            looping: false,
        }
    }
}

#[cfg(feature = "audio")]
mod backend {
    use std::collections::HashMap;
    use std::sync::Arc;

    use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
    use kira::sound::{FromFileError, PlaybackState};
    use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Tween};

    use super::SoundParams;

    #[derive(Default)]
    pub struct Audio {
        /// `None` until first use; `Some(None)` if there's no device.
        manager: Option<Option<AudioManager<DefaultBackend>>>,
        sounds: Vec<StaticSoundData>,
        playing: HashMap<u64, StaticSoundHandle>,
        next: u64,
    }

    impl Audio {
        fn manager(&mut self) -> Option<&mut AudioManager<DefaultBackend>> {
            self.manager
                .get_or_insert_with(|| match AudioManager::new(AudioManagerSettings::default()) {
                    Ok(m) => Some(m),
                    Err(e) => {
                        eprintln!("crabbyengine: no audio ({e}); sounds will be silent");
                        None
                    }
                })
                .as_mut()
        }

        pub fn load(&mut self, bytes: Vec<u8>) -> Result<(u32, f32), String> {
            let data = StaticSoundData::from_cursor(std::io::Cursor::new(bytes)).map_err(|e| match e {
                FromFileError::UnsupportedChannelConfiguration => "only mono and stereo sounds are supported".into(),
                other => format!("could not decode the sound: {other}"),
            })?;
            Ok(self.add(data))
        }

        pub fn create(&mut self, sample_rate: u32, samples: &[f32]) -> (u32, f32) {
            let frames: Arc<[Frame]> = samples.iter().map(|&s| Frame::from_mono(s)).collect();
            self.add(StaticSoundData {
                sample_rate,
                frames,
                settings: Default::default(),
                slice: None,
            })
        }

        fn add(&mut self, data: StaticSoundData) -> (u32, f32) {
            let seconds = data.duration().as_secs_f32();
            self.sounds.push(data);
            ((self.sounds.len() - 1) as u32, seconds)
        }

        pub fn play(&mut self, sound: u32, p: SoundParams) -> u64 {
            // Forget sounds that finished, so the map stays small.
            self.playing.retain(|_, h| h.state() != PlaybackState::Stopped);
            let mut data = self.sounds[sound as usize]
                .volume(volume_db(p.volume))
                .playback_rate(f64::from(p.speed.max(0.01)));
            if p.looping {
                data = data.loop_region(..);
            }
            let id = self.next;
            self.next += 1;
            if let Some(m) = self.manager()
                && let Ok(handle) = m.play(data)
            {
                self.playing.insert(id, handle);
            }
            id
        }

        pub fn stop(&mut self, id: u64) {
            if let Some(mut h) = self.playing.remove(&id) {
                h.stop(Tween::default());
            }
        }
    }

    /// Linear volume (0..1) to decibels.
    fn volume_db(v: f32) -> Decibels {
        if v <= 0.001 {
            Decibels::SILENCE
        } else {
            Decibels(20.0 * v.log10())
        }
    }
}

#[cfg(not(feature = "audio"))]
mod backend {
    use super::SoundParams;

    /// The `audio` feature is off: sounds have a length but make no noise.
    #[derive(Default)]
    pub struct Audio {
        count: u32,
        next: u64,
    }

    impl Audio {
        pub fn load(&mut self, _bytes: Vec<u8>) -> Result<(u32, f32), String> {
            self.count += 1;
            Ok((self.count - 1, 0.0))
        }

        pub fn create(&mut self, sample_rate: u32, samples: &[f32]) -> (u32, f32) {
            self.count += 1;
            (self.count - 1, samples.len() as f32 / sample_rate.max(1) as f32)
        }

        pub fn play(&mut self, _sound: u32, _p: SoundParams) -> u64 {
            self.next += 1;
            self.next
        }

        pub fn stop(&mut self, _id: u64) {}
    }
}

pub(crate) use backend::Audio;

impl Ctx {
    /// Loads a sound from the `assets/` folder: OGG Vorbis, WAV, MP3 or FLAC.
    /// Loading the same file again returns the same handle.
    ///
    /// # Panics
    /// With a clear message if the file is missing or can't be decoded.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let hit = ctx.load_sound("hit.ogg");
    /// ctx.play_sound(&hit);
    /// # }
    /// ```
    pub fn load_sound(&mut self, path: &str) -> Sound {
        let file = crate::assets::resolve(path).unwrap_or_else(|e| panic!("crabbyengine: {e}"));
        let key = (file, crate::ctx::AssetKind::Sound);
        if let Some(&i) = self.loaded.get(&key) {
            return self.cached_sounds[i as usize];
        }
        let bytes =
            std::fs::read(&key.0).unwrap_or_else(|e| panic!("crabbyengine: could not read {}: {e}", key.0.display()));
        let (id, seconds) = self
            .audio
            .load(bytes)
            .unwrap_or_else(|e| panic!("crabbyengine: {}: {e}", key.0.display()));
        let sound = Sound { id, seconds };
        self.loaded.insert(key, self.cached_sounds.len() as u64);
        self.cached_sounds.push(sound);
        sound
    }

    /// Makes a sound from mono samples in -1..1 at `sample_rate` Hz, for
    /// generated effects.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// // A short 880 Hz beep that fades out.
    /// let rate = 44_100;
    /// let beep: Vec<f32> = (0..rate / 10)
    ///     .map(|i| {
    ///         let t = i as f32 / rate as f32;
    ///         (t * 880.0 * std::f32::consts::TAU).sin() * (1.0 - t * 10.0) * 0.3
    ///     })
    ///     .collect();
    /// let beep = ctx.create_sound(rate, &beep);
    /// assert!((beep.seconds() - 0.1).abs() < 0.001);
    /// # }
    /// # Game::new("t", 1, 1).run_headless(0, setup);
    /// ```
    pub fn create_sound(&mut self, sample_rate: u32, samples: &[f32]) -> Sound {
        assert!(sample_rate > 0, "crabbyengine: the sample rate must not be zero");
        let (id, seconds) = self.audio.create(sample_rate, samples);
        Sound { id, seconds }
    }

    /// Plays `sound` once at full volume. Silent in headless runs.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, click: Sound) {
    /// if ctx.mouse_pressed(MouseButton::Left) {
    ///     ctx.play_sound(&click);
    /// }
    /// # }
    /// ```
    pub fn play_sound(&mut self, sound: &Sound) -> Playing {
        self.play_sound_ex(sound, SoundParams::default())
    }

    /// Plays `sound` with a volume, speed, or looping.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, step: Sound) {
    /// let speed = 0.9 + ctx.random().stream(0).f32() * 0.2; // vary the pitch a little
    /// ctx.play_sound_ex(&step, SoundParams { volume: 0.4, speed, ..Default::default() });
    /// # }
    /// ```
    pub fn play_sound_ex(&mut self, sound: &Sound, params: SoundParams) -> Playing {
        if self.headless {
            return Playing(0);
        }
        Playing(self.audio.play(sound.id, params))
    }

    /// Stops a sound started with [`play_sound`](Self::play_sound) (with a
    /// short fade). Does nothing if it already finished.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, music: Sound) {
    /// let song = ctx.play_sound_ex(&music, SoundParams { looping: true, ..Default::default() });
    /// // ... later
    /// ctx.stop_sound(song);
    /// # }
    /// ```
    pub fn stop_sound(&mut self, playing: Playing) {
        self.audio.stop(playing.0);
    }
}
