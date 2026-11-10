//! [`Audio`], the sound service at `ctx.audio`: sound effects and music,
//! played with `kira`.
//!
//! **What lives here:** loading and creating sounds ([`Audio::load`],
//! [`Audio::create`], [`Audio::load_stream`]), playing them
//! ([`Audio::play`], [`Audio::play_ex`], [`Audio::play_on`], and
//! [`Audio::play_at`] for a point in the 3D world), the handles of what is
//! playing ([`Audio::stop`], [`Audio::set_effects`]), mixer buses
//! ([`Audio::bus`], [`Audio::set_bus_volume`], [`Audio::mute_bus`]) and the
//! 3D listener ([`Audio::set_listener`]).
//!
//! **When to call it:** load in setup and keep the [`Sound`] handles; play,
//! stop and mix from any system. It only borrows `ctx.audio`, so a sound can
//! be played while iterating `ctx.ui.events()` or a `ctx.world` query.
//!
//! **What it needs:** the `audio` feature and an output device to be heard.
//! The device opens the first time a sound is loaded or played. If there is
//! none (servers, CI), the feature is off, or the run is headless,
//! everything here still works and is simply silent, so game code never
//! needs a special case. Positional sound needs the `3d` feature.

#[cfg(feature = "3d")]
mod spatial;
#[cfg(feature = "3d")]
pub use spatial::*;

/// A loaded sound. Cheap to copy. Get one from
/// [`Ctx::load_sound`](crate::Ctx::load_sound) or
/// [`Audio::create`](crate::Audio::create).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Sound {
    pub(crate) id: u32,
    /// Length in seconds.
    seconds: f32,
}

// A sound's length is never NaN, so equality is an equivalence.
impl Eq for Sound {}

impl std::hash::Hash for Sound {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Equal sounds have equal ids; the length comes with the id.
        self.id.hash(state);
    }
}

impl Sound {
    pub(crate) const fn new(id: u32, seconds: f32) -> Self {
        Self { id, seconds }
    }

    /// Length in seconds.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(sound: Sound) {
    /// println!("{:.1} s", sound.seconds());
    /// # }
    /// ```
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.seconds
    }
}

/// A sound that was started, to stop it later with
/// [`Audio::stop`](crate::Audio::stop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Playing(pub(crate) u64);

/// Optional settings for [`Audio::play_ex`](crate::Audio::play_ex).
///
/// ```
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx, music: Sound) {
/// ctx.audio.play_ex(&music, SoundParams { volume: 0.5, looping: true, ..Default::default() });
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

mod mixer;
use mixer::BusRegistry;
pub use mixer::{AudioBus, AudioEffects};

#[cfg(feature = "audio")]
mod backend {
    use super::mixer::{finite, volume_db};
    use super::{AudioBus, AudioEffects, BusRegistry, SoundParams};
    use kira::effect::{
        filter::{FilterBuilder, FilterHandle},
        reverb::ReverbBuilder,
    };
    use kira::sound::{
        PlaybackState,
        static_sound::{StaticSoundData, StaticSoundHandle},
    };
    // Kira streams from files, which the web doesn't have.
    #[cfg(not(target_arch = "wasm32"))]
    use kira::sound::{
        FromFileError,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    };
    use kira::track::{SendTrackBuilder, SendTrackHandle, TrackBuilder, TrackHandle};
    use kira::{AudioManager, AudioManagerSettings, DefaultBackend, Frame, Tween};
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    enum Clip {
        Static(Box<StaticSoundData>),
        #[cfg(not(target_arch = "wasm32"))]
        Stream(StreamFrom),
        /// Freed: plays nothing, and its id is never reused.
        Freed,
    }
    enum Handle {
        Static(StaticSoundHandle),
        #[cfg(not(target_arch = "wasm32"))]
        Stream(StreamingSoundHandle<FromFileError>),
    }
    struct Voice {
        /// The sound it plays.
        sound: u32,
        handle: Handle,
        track: TrackHandle,
        filter: FilterHandle,
        bus: AudioBus,
        effects: AudioEffects,
    }
    impl Handle {
        fn state(&self) -> PlaybackState {
            match self {
                Self::Static(h) => h.state(),
                #[cfg(not(target_arch = "wasm32"))]
                Self::Stream(h) => h.state(),
            }
        }
        fn stop(&mut self) {
            match self {
                Self::Static(h) => h.stop(Tween::default()),
                #[cfg(not(target_arch = "wasm32"))]
                Self::Stream(h) => h.stop(Tween::default()),
            }
        }
        #[cfg(feature = "3d")]
        fn spatialize(&mut self, volume: f32, pan: f32, speed: f32) {
            macro_rules! apply {
                ($h:expr) => {{
                    $h.set_volume(volume_db(volume), Tween::default());
                    $h.set_panning(pan, Tween::default());
                    $h.set_playback_rate(f64::from(finite(speed, 1.0).max(0.01)), Tween::default());
                }};
            }
            match self {
                Self::Static(h) => apply!(h),
                #[cfg(not(target_arch = "wasm32"))]
                Self::Stream(h) => apply!(h),
            }
        }
    }
    /// Where a streamed sound's encoded bytes are: each playback opens its
    /// own decoder over them.
    #[cfg(not(target_arch = "wasm32"))]
    enum StreamFrom {
        /// A file, opened for each playback.
        File(PathBuf),
        /// Encoded bytes an asset source handed over, kept (not decoded) in
        /// memory and shared by every playback.
        Memory(Arc<[u8]>),
    }
    #[cfg(not(target_arch = "wasm32"))]
    impl StreamFrom {
        fn open(&self) -> Result<StreamingSoundData<FromFileError>, FromFileError> {
            match self {
                Self::File(path) => StreamingSoundData::from_file(path),
                Self::Memory(bytes) => StreamingSoundData::from_cursor(std::io::Cursor::new(bytes.clone())),
            }
        }
    }
    #[derive(Default)]
    pub struct Audio {
        manager: Option<Option<AudioManager<DefaultBackend>>>,
        sounds: Vec<Clip>,
        playing: HashMap<u64, Voice>,
        next: u64,
        pub(super) buses: BusRegistry,
        reverb: Option<SendTrackHandle>,
        #[cfg(feature = "3d")]
        pub(crate) spatial: super::spatial::SpatialState,
    }
    impl Audio {
        /// For tools: whether the output device was opened (it opens on first use).
        pub(crate) fn device_state(&self) -> &'static str {
            match &self.manager {
                None => "not_opened",
                Some(None) => "failed",
                Some(Some(_)) => "open",
            }
        }
        fn manager(&mut self) -> Option<&mut AudioManager<DefaultBackend>> {
            self.manager
                .get_or_insert_with(|| AudioManager::new(AudioManagerSettings::default()).ok())
                .as_mut()
        }
        pub fn load(&mut self, bytes: Vec<u8>) -> Result<(u32, f32), String> {
            Ok(self.add(decode(bytes)?))
        }
        /// Sounds made and not freed, and the bytes they hold: decoded
        /// samples, or for a sound streamed from memory its encoded bytes
        /// (one streamed from a file is read as it plays: 0).
        pub fn inventory(&self) -> (usize, u64) {
            let live = self.sounds.iter().filter(|clip| !matches!(clip, Clip::Freed));
            let bytes = live
                .clone()
                .map(|clip| match clip {
                    Clip::Static(data) => (data.frames.len() * std::mem::size_of::<Frame>()) as u64,
                    #[cfg(not(target_arch = "wasm32"))]
                    Clip::Stream(StreamFrom::File(_)) => 0,
                    #[cfg(not(target_arch = "wasm32"))]
                    Clip::Stream(StreamFrom::Memory(bytes)) => bytes.len() as u64,
                    Clip::Freed => 0,
                })
                .sum();
            (live.count(), bytes)
        }
        pub fn replace(&mut self, id: u32, bytes: Vec<u8>) -> Result<(), String> {
            let data = decode(bytes)?;
            if let Some(clip) = self.sounds.get_mut(id as usize)
                && !matches!(clip, Clip::Freed)
            {
                *clip = Clip::Static(Box::new(data));
            }
            Ok(())
        }
        /// Drops sound `id`'s data and stops its voices.
        pub fn free(&mut self, id: u32) {
            if let Some(clip) = self.sounds.get_mut(id as usize) {
                *clip = Clip::Freed;
            }
            for voice in self.playing.values_mut().filter(|v| v.sound == id) {
                voice.handle.stop();
            }
            self.playing.retain(|_, v| v.sound != id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        pub fn stream(&mut self, path: PathBuf) -> Result<(u32, f32), String> {
            if let Some(index) = self
                .sounds
                .iter()
                .position(|clip| matches!(clip, Clip::Stream(StreamFrom::File(p)) if *p == path))
            {
                let seconds = StreamingSoundData::from_file(&path)
                    .map_err(|e| e.to_string())?
                    .duration()
                    .as_secs_f32();
                return Ok((index as u32, seconds));
            }
            let data = StreamingSoundData::from_file(&path).map_err(|e| e.to_string())?;
            let seconds = data.duration().as_secs_f32();
            self.sounds.push(Clip::Stream(StreamFrom::File(path)));
            Ok(((self.sounds.len() - 1) as u32, seconds))
        }
        /// A sound streamed from encoded `bytes` held in memory (an asset
        /// source's): only the header is decoded now, the samples as it plays.
        #[cfg(not(target_arch = "wasm32"))]
        pub fn stream_bytes(&mut self, bytes: Vec<u8>) -> Result<(u32, f32), String> {
            let from = StreamFrom::Memory(bytes.into());
            let seconds = from
                .open()
                .map_err(|e| format!("could not decode the sound: {e}"))?
                .duration()
                .as_secs_f32();
            self.sounds.push(Clip::Stream(from));
            Ok(((self.sounds.len() - 1) as u32, seconds))
        }
        /// The web can't stream from files: read the fetched copy and decode it whole.
        #[cfg(target_arch = "wasm32")]
        pub fn stream(&mut self, path: PathBuf) -> Result<(u32, f32), String> {
            self.load(crate::assets::read(&path).map_err(|e| e.to_string())?)
        }
        /// The web decodes streamed sounds whole (no decoder thread).
        #[cfg(target_arch = "wasm32")]
        pub fn stream_bytes(&mut self, bytes: Vec<u8>) -> Result<(u32, f32), String> {
            self.load(bytes)
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
            self.sounds.push(Clip::Static(Box::new(data)));
            ((self.sounds.len() - 1) as u32, seconds)
        }
        fn prepare(&mut self, bus: AudioBus) -> Option<()> {
            self.buses.ensure_default();
            self.buses.entries.get(bus.0)?;
            if self.reverb.is_none() {
                self.reverb = Some(
                    self.manager()?
                        .add_send_track(SendTrackBuilder::new().with_effect(ReverbBuilder::new().mix(1.0)))
                        .ok()?,
                );
            }
            if self.buses.entries[bus.0].track.is_none() {
                let gain = self.buses.entries[bus.0].gain();
                let track = self
                    .manager()?
                    .add_sub_track(TrackBuilder::new().volume(volume_db(gain)))
                    .ok()?;
                self.buses.entries[bus.0].track = Some(track);
            }
            Some(())
        }
        pub fn play(&mut self, sound: u32, p: SoundParams) -> u64 {
            self.play_on(sound, p, AudioBus::default(), AudioEffects::default())
        }
        pub fn play_on(&mut self, sound: u32, p: SoundParams, bus: AudioBus, effects: AudioEffects) -> u64 {
            self.playing
                .retain(|_, voice| voice.handle.state() != PlaybackState::Stopped);
            self.next += 1;
            let id = self.next;
            if self.prepare(bus).is_none() {
                return id;
            }
            let effects = effects.validated();
            let send = self.reverb.as_ref().unwrap().id();
            let gain = self.buses.entries[bus.0].gain();
            let mut builder = TrackBuilder::new().with_send(send, volume_db(effects.reverb * gain));
            let filter = builder.add_effect(FilterBuilder::new().cutoff(f64::from(effects.low_pass_hz)));
            let Ok(mut track) = self.buses.entries[bus.0].track.as_mut().unwrap().add_sub_track(builder) else {
                return id;
            };
            let volume = volume_db(p.volume);
            let speed = f64::from(finite(p.speed, 1.0).max(0.01));
            let Some(clip) = self.sounds.get(sound as usize) else {
                return id;
            };
            let handle = match clip {
                Clip::Freed => return id,
                Clip::Static(data) => {
                    let mut data = (**data).clone().volume(volume).playback_rate(speed);
                    if p.looping {
                        data = data.loop_region(..);
                    }
                    track.play(data).ok().map(Handle::Static)
                }
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Stream(from) => {
                    let Ok(data) = from.open() else {
                        return id;
                    };
                    let mut data = data.volume(volume).playback_rate(speed);
                    if p.looping {
                        data = data.loop_region(..);
                    }
                    track.play(data).ok().map(Handle::Stream)
                }
            };
            if let Some(handle) = handle {
                self.playing.insert(
                    id,
                    Voice {
                        sound,
                        handle,
                        track,
                        filter,
                        bus,
                        effects,
                    },
                );
            }
            id
        }
        pub fn bus_changed(&mut self, bus: AudioBus) {
            let Some(state) = self.buses.entries.get_mut(bus.0) else {
                return;
            };
            let gain = state.gain();
            if let Some(track) = state.track.as_mut() {
                track.set_volume(volume_db(gain), Tween::default());
            }
            if let Some(send) = &self.reverb {
                for voice in self.playing.values_mut().filter(|v| v.bus == bus) {
                    let _ = voice
                        .track
                        .set_send(send.id(), volume_db(voice.effects.reverb * gain), Tween::default());
                }
            }
        }
        pub fn effects(&mut self, id: u64, effects: AudioEffects) {
            if let Some(voice) = self.playing.get_mut(&id) {
                let effects = effects.validated();
                voice
                    .filter
                    .set_cutoff(f64::from(effects.low_pass_hz), Tween::default());
                if let Some(send) = &self.reverb {
                    let gain = self.buses.entries[voice.bus.0].gain();
                    let _ = voice
                        .track
                        .set_send(send.id(), volume_db(effects.reverb * gain), Tween::default());
                }
                voice.effects = effects;
            }
        }
        #[cfg(feature = "3d")]
        pub fn spatialize(&mut self, id: u64, volume: f32, pan: f32, speed: f32) {
            if let Some(v) = self.playing.get_mut(&id) {
                v.handle.spatialize(volume, pan, speed);
            }
        }
        pub fn stop(&mut self, id: u64) {
            if let Some(mut v) = self.playing.remove(&id) {
                v.handle.stop();
            }
        }
    }
    fn decode(bytes: Vec<u8>) -> Result<StaticSoundData, String> {
        StaticSoundData::from_cursor(std::io::Cursor::new(bytes))
            .map_err(|e| format!("could not decode the sound: {e}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn audio_stream_loads_headers_reuses_handle_and_never_opens_device() {
            let path = std::env::temp_dir().join(format!(
                "ponko-audio-{}-{}.wav",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let mut wav = Vec::from(&b"RIFF"[..]);
            wav.extend_from_slice(&236u32.to_le_bytes());
            wav.extend_from_slice(b"WAVEfmt ");
            wav.extend_from_slice(&16u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
            wav.extend_from_slice(&1u16.to_le_bytes()); // mono
            wav.extend_from_slice(&100u32.to_le_bytes());
            wav.extend_from_slice(&200u32.to_le_bytes());
            wav.extend_from_slice(&2u16.to_le_bytes());
            wav.extend_from_slice(&16u16.to_le_bytes());
            wav.extend_from_slice(b"data");
            wav.extend_from_slice(&200u32.to_le_bytes());
            wav.resize(244, 0);
            std::fs::write(&path, wav).unwrap();
            let mut audio = Audio::default();
            let first = audio.stream(path.clone());
            let second = audio.stream(path.clone());
            std::fs::remove_file(path).unwrap();
            assert_eq!(first.unwrap(), second.unwrap());
            assert_eq!(audio.sounds.len(), 1);
            assert!(audio.manager.is_none());
            assert!(matches!(&audio.sounds[0], Clip::Stream(_)));
        }

        #[test]
        fn audio_headless_routing_never_initializes_manager() {
            crate::Game::new("audio", 1, 1).run_headless(0, |ctx| {
                let bus = ctx.audio.bus("music");
                let sound = ctx.audio.create(100, &[0.0; 100]);
                let voice = ctx
                    .audio
                    .play_on(&sound, SoundParams::default(), bus, AudioEffects::default());
                ctx.audio.set_effects(voice, AudioEffects::default());
                ctx.audio.set_bus_volume(bus, 0.5);
                ctx.audio.mute_bus(bus, true);
                assert!(ctx.audio.backend.manager.is_none());
                assert!(ctx.audio.backend.playing.is_empty());
                assert!(ctx.audio.backend.reverb.is_none());
            });
        }
    }
}

#[cfg(not(feature = "audio"))]
mod backend {
    use super::{AudioBus, AudioEffects, BusRegistry, SoundParams};
    #[derive(Default)]
    pub struct Audio {
        count: u32,
        /// Sounds freed (each id once: `Ctx::free` checks).
        freed: u32,
        next: u64,
        pub(super) buses: BusRegistry,
        #[cfg(feature = "3d")]
        pub(crate) spatial: super::spatial::SpatialState,
    }
    impl Audio {
        /// For tools: this build has no audio output.
        pub(crate) fn device_state(&self) -> &'static str {
            "compiled_out"
        }
        pub fn load(&mut self, _bytes: Vec<u8>) -> Result<(u32, f32), String> {
            self.count += 1;
            Ok((self.count - 1, 0.0))
        }
        /// Sounds made and not freed; nothing is decoded without the
        /// `audio` feature.
        pub fn inventory(&self) -> (usize, u64) {
            ((self.count - self.freed) as usize, 0)
        }
        pub fn stream(&mut self, _path: std::path::PathBuf) -> Result<(u32, f32), String> {
            self.load(Vec::new())
        }
        pub fn stream_bytes(&mut self, _bytes: Vec<u8>) -> Result<(u32, f32), String> {
            self.load(Vec::new())
        }
        pub fn replace(&mut self, _id: u32, _bytes: Vec<u8>) -> Result<(), String> {
            Ok(())
        }
        pub fn free(&mut self, _id: u32) {
            self.freed += 1;
        }
        pub fn create(&mut self, sample_rate: u32, samples: &[f32]) -> (u32, f32) {
            self.count += 1;
            (self.count - 1, samples.len() as f32 / sample_rate.max(1) as f32)
        }
        pub fn play(&mut self, _sound: u32, _p: SoundParams) -> u64 {
            self.next += 1;
            self.next
        }
        pub fn play_on(&mut self, sound: u32, p: SoundParams, _bus: AudioBus, _effects: AudioEffects) -> u64 {
            self.play(sound, p)
        }
        pub fn bus_changed(&mut self, _bus: AudioBus) {}
        pub fn effects(&mut self, _id: u64, _effects: AudioEffects) {}
        #[cfg(feature = "3d")]
        pub fn spatialize(&mut self, _id: u64, _volume: f32, _pan: f32, _speed: f32) {}
        pub fn stop(&mut self, _id: u64) {}
    }
}

pub(crate) use backend::Audio as Backend;

/// Sounds, playing handles, buses and the listener: `ctx.audio`.
///
/// `ctx.load_sound` and `ctx.play_sound` are shortcuts for
/// [`load`](Self::load) and [`play`](Self::play); everything else is here.
///
/// - **What lives here:** loading and creating sounds, playing them (also
///   at a point in the 3D world), the handles of what is playing, mixer
///   buses and the 3D listener.
/// - **When to call it:** load in setup and keep the [`Sound`] handles;
///   play, stop and mix from any system. It borrows only `ctx.audio`, so a
///   sound can play while `ctx.ui.events()` or a `ctx.world` query is
///   being read.
/// - **What it needs:** the `audio` feature and an output device to be
///   heard. Without them, and in headless runs, every call still works and
///   is silent. Positional sound needs the `3d` feature.
///
/// ```
/// use ponko::*;
///
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     let beep = ctx.audio.create(8000, &[0.0; 800]);
///     let music = ctx.audio.bus("music");
///     ctx.audio.set_bus_volume(music, 0.5);
///     let playing = ctx.audio.play_on(&beep, SoundParams::default(), music, AudioEffects::default());
///     ctx.audio.stop(playing);
/// });
/// ```
#[derive(Default)]
pub struct Audio {
    /// The mixer: kira, or the silent stand-in without the `audio` feature.
    pub(crate) backend: Backend,
    /// The game's sounds, by id, live or freed.
    pub(crate) sounds_live: crate::handle::LiveSet,
    /// Sounds loaded from files, indexed by `loaded` values.
    pub(crate) cached_sounds: Vec<Sound>,
    /// Sounds already loaded, by origin: loading one again returns the same handle.
    pub(crate) loaded: std::collections::HashMap<(crate::assets::Origin, crate::ctx::AssetKind), u64>,
    /// Where named assets come from, shared with the other loading services.
    pub(crate) asset_sources: crate::assets::SharedSources,
    /// Hot reload: which files sounds came from.
    pub(crate) watcher: crate::hot_reload::SharedWatcher,
    /// No window: nothing is played.
    pub(crate) headless: bool,
}

impl Audio {
    /// Loads a sound from an [added asset source](crate::Assets::add_source) or the
    /// `assets/` folder: OGG Vorbis, WAV, MP3 or FLAC. Loading the same file
    /// again returns the same handle. Only a file from `assets/` is hot reloaded.
    ///
    /// Without the `audio` feature nothing is decoded: the file only has to
    /// exist, and the sound lasts zero seconds.
    ///
    /// # Panics
    /// With a clear message if the file is missing or, with the `audio`
    /// feature, can't be decoded.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let hit = ctx.load_sound("hit.ogg");
    /// ctx.play_sound(&hit);
    /// # }
    /// ```
    pub fn load(&mut self, path: &str) -> Sound {
        self.try_load(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// [`load`](Self::load) with errors as values:
    /// [`ErrorKind::NotFound`](crate::ErrorKind::NotFound),
    /// [`Io`](crate::ErrorKind::Io), or (with the `audio` feature)
    /// [`Decode`](crate::ErrorKind::Decode) if it can't be decoded.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     if let Ok(music) = ctx.audio.try_load("music/bonus.ogg") {
    ///         ctx.play_sound(&music);
    ///     }
    /// });
    /// ```
    pub fn try_load(&mut self, path: &str) -> Result<Sound, crate::Error> {
        let found = match self.asset_sources.lookup(path, |origin| {
            let key = (origin.clone(), crate::ctx::AssetKind::Sound);
            self.loaded.get(&key).map(|&i| self.cached_sounds[i as usize])
        })? {
            crate::assets::Lookup::Cached(_, sound) => return Ok(sound),
            crate::assets::Lookup::Found(found) => found,
        };
        let origin = found.origin.clone();
        let bytes = found.into_bytes().map_err(|e| e.with_path(path))?;
        let (id, seconds) = self.backend.load(bytes).map_err(|e| match &origin {
            crate::assets::Origin::File(file) => crate::Error::decode(path, format!("{}: {e}", file.display())),
            crate::assets::Origin::Source { .. } => {
                crate::Error::decode(path, format!("{}: {e}", origin.describe(path)))
            }
        })?;
        let sound = Sound { id, seconds };
        self.sounds_live.insert(id);
        self.loaded.insert(
            (origin.clone(), crate::ctx::AssetKind::Sound),
            self.cached_sounds.len() as u64,
        );
        self.cached_sounds.push(sound);
        if let crate::assets::Origin::File(file) = origin {
            self.watcher.watch(path, file, crate::hot_reload::Asset::Sound(id));
        }
        Ok(sound)
    }

    /// Makes a sound from mono samples in -1..1 at `sample_rate` Hz, for
    /// generated effects.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// // A short 880 Hz beep that fades out.
    /// let rate = 44_100;
    /// let beep: Vec<f32> = (0..rate / 10)
    ///     .map(|i| {
    ///         let t = i as f32 / rate as f32;
    ///         (t * 880.0 * std::f32::consts::TAU).sin() * (1.0 - t * 10.0) * 0.3
    ///     })
    ///     .collect();
    /// let beep = ctx.audio.create(rate, &beep);
    /// assert!((beep.seconds() - 0.1).abs() < 0.001);
    /// # }
    /// # Game::new("t", 1, 1).run_headless(0, setup);
    /// ```
    ///
    /// # Panics
    ///
    /// If `sample_rate` is 0.
    pub fn create(&mut self, sample_rate: u32, samples: &[f32]) -> Sound {
        assert!(sample_rate > 0, "ponko: the sample rate must not be zero");
        let (id, seconds) = self.backend.create(sample_rate, samples);
        self.sounds_live.insert(id);
        Sound { id, seconds }
    }

    /// Plays `sound` once at full volume. Silent in headless runs.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, click: Sound) {
    /// if ctx.input.mouse_pressed(MouseButton::Left) {
    ///     ctx.play_sound(&click);
    /// }
    /// # }
    /// ```
    pub fn play(&mut self, sound: &Sound) -> Playing {
        self.play_ex(sound, SoundParams::default())
    }

    /// Plays `sound` with a volume, speed, or looping.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, step: Sound) {
    /// let speed = 0.9 + ctx.random().stream(0).f32() * 0.2; // vary the pitch a little
    /// ctx.audio.play_ex(&step, SoundParams { volume: 0.4, speed, ..Default::default() });
    /// # }
    /// ```
    pub fn play_ex(&mut self, sound: &Sound, params: SoundParams) -> Playing {
        if self.headless {
            return Playing(0);
        }
        Playing(self.backend.play(sound.id, params))
    }

    /// Frees sound `id` (see [`Ctx::free`]).
    pub(crate) fn free_id(&mut self, id: u32) {
        if !self.sounds_live.remove(id) {
            return;
        }
        let cached = &self.cached_sounds;
        self.loaded.retain(|(_, kind), i| {
            !matches!(kind, crate::ctx::AssetKind::Sound | crate::ctx::AssetKind::Stream)
                || cached[*i as usize].id != id
        });
        self.watcher
            .forget(|a| matches!(a, crate::hot_reload::Asset::Sound(s) if *s == id));
        self.backend.free(id);
    }

    /// Stops a sound started with [`play`](Self::play) (with a
    /// short fade). Does nothing if it already finished.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, music: Sound) {
    /// let song = ctx.audio.play_ex(&music, SoundParams { looping: true, ..Default::default() });
    /// // ... later
    /// ctx.audio.stop(song);
    /// # }
    /// ```
    pub fn stop(&mut self, playing: Playing) {
        #[cfg(feature = "3d")]
        self.backend.spatial.remove(playing);
        self.backend.stop(playing.0);
    }
}
