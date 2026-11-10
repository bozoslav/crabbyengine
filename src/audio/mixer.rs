use super::{Audio, Playing, Sound, SoundParams};

/// Named mixer bus, created once with [`Audio::bus`]. Handles belong to
/// their creating context. The default handle selects the `sfx` bus.
/// ```
/// use ponko::AudioBus;
/// let bus = AudioBus::default();
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)] // the C API mirrors it as a `size_t`
pub struct AudioBus(pub(super) usize);

/// Per-voice Kira low-pass and shared reverb send settings. Invalid values
/// become defaults; cutoff is clamped to 20–20,000 Hz and send to 0–1.
/// ```
/// use ponko::AudioEffects;
/// let underwater = AudioEffects { low_pass_hz: 800.0, reverb: 0.3 };
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioEffects {
    /// Low-pass cutoff in Hz. Default 20,000.
    /// ```
    /// # use ponko::AudioEffects;
    /// assert_eq!(AudioEffects::default().low_pass_hz, 20_000.0);
    /// ```
    pub low_pass_hz: f32,
    /// Linear send gain to a shared, fully wet Kira reverb. Default zero.
    /// ```
    /// # use ponko::AudioEffects;
    /// assert_eq!(AudioEffects::default().reverb, 0.0);
    /// ```
    pub reverb: f32,
}
impl Default for AudioEffects {
    fn default() -> Self {
        Self {
            low_pass_hz: 20_000.0,
            reverb: 0.0,
        }
    }
}
impl AudioEffects {
    pub(super) fn validated(self) -> Self {
        Self {
            low_pass_hz: finite(self.low_pass_hz, 20_000.0).clamp(20.0, 20_000.0),
            reverb: finite(self.reverb, 0.0).clamp(0.0, 1.0),
        }
    }
}
pub(super) fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
#[cfg(feature = "audio")]
pub(super) fn volume_db(value: f32) -> kira::Decibels {
    let value = finite(value, 0.0).max(0.0);
    if value <= 0.001 {
        kira::Decibels::SILENCE
    } else {
        kira::Decibels(20.0 * value.log10())
    }
}
pub(super) struct BusState {
    name: String,
    volume: f32,
    muted: bool,
    #[cfg(feature = "audio")]
    pub(super) track: Option<kira::track::TrackHandle>,
}
impl BusState {
    #[cfg(feature = "audio")]
    pub(super) fn gain(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }
}
#[derive(Default)]
pub(super) struct BusRegistry {
    pub(super) entries: Vec<BusState>,
}
impl BusRegistry {
    pub(super) fn ensure_default(&mut self) {
        if self.entries.is_empty() {
            self.insert("sfx");
        }
    }
    fn insert(&mut self, name: &str) -> AudioBus {
        let id = AudioBus(self.entries.len());
        self.entries.push(BusState {
            name: name.into(),
            volume: 1.0,
            muted: false,
            #[cfg(feature = "audio")]
            track: None,
        });
        id
    }
    fn named(&mut self, name: &str) -> AudioBus {
        self.ensure_default();
        if let Some(index) = self.entries.iter().position(|bus| bus.name == name) {
            AudioBus(index)
        } else {
            self.insert(name)
        }
    }
}
impl Audio {
    /// Gets or creates a named bus. Lookup allocates only for a new name;
    /// save the returned handle. This never opens an audio device.
    /// ```
    /// use ponko::*;
    /// Game::new("audio", 1, 1).run_headless(0, |ctx| {
    ///     assert_eq!(ctx.audio.bus("music"), ctx.audio.bus("music"));
    /// });
    /// ```
    pub fn bus(&mut self, name: &str) -> AudioBus {
        self.backend.buses.named(name)
    }

    /// Changes a bus's linear gain, preserving its mute state. Negative or
    /// nonfinite values become silence. Finished reverb tails decay naturally.
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let music = ctx.audio.bus("music");
    /// ctx.audio.set_bus_volume(music, 0.5);
    /// # }
    /// ```
    pub fn set_bus_volume(&mut self, bus: AudioBus, volume: f32) {
        self.backend.buses.ensure_default();
        if let Some(state) = self.backend.buses.entries.get_mut(bus.0) {
            state.volume = finite(volume, 0.0).max(0.0);
        }
        self.backend.bus_changed(bus);
    }
    /// Mutes or unmutes a bus without losing its configured volume. Existing
    /// reverb tails decay naturally; new input to reverb is muted immediately.
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let music = ctx.audio.bus("music");
    /// ctx.audio.mute_bus(music, true);
    /// # }
    /// ```
    pub fn mute_bus(&mut self, bus: AudioBus, muted: bool) {
        self.backend.buses.ensure_default();
        if let Some(state) = self.backend.buses.entries.get_mut(bus.0) {
            state.muted = muted;
        }
        self.backend.bus_changed(bus);
    }
    /// Plays a decoded or streamed sound on a bus with per-voice effects.
    /// Ordinary `play_sound` and `play_sound_ex` use `sfx` and default effects.
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, sound: Sound) {
    /// let music = ctx.audio.bus("music");
    /// ctx.audio.play_on(&sound, SoundParams::default(), music, AudioEffects::default());
    /// # }
    /// ```
    pub fn play_on(&mut self, sound: &Sound, params: SoundParams, bus: AudioBus, effects: AudioEffects) -> Playing {
        if self.headless {
            return Playing(0);
        }
        Playing(self.backend.play_on(sound.id, params, bus, effects.validated()))
    }
    /// Changes a playing voice's low-pass cutoff and reverb send with Kira
    /// smoothing. Does nothing for stopped or headless voices.
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, voice: Playing) {
    /// ctx.audio.set_effects(voice, AudioEffects { low_pass_hz: 700.0, reverb: 0.4 });
    /// # }
    /// ```
    pub fn set_effects(&mut self, playing: Playing, effects: AudioEffects) {
        self.backend.effects(playing.0, effects.validated());
    }

    /// Loads music to stream, from an [added asset
    /// source](crate::Assets::add_source) or `assets/`, returning the same kind
    /// of handle as `load_sound`. Each playback opens its own decoder:
    /// samples are decoded on Kira's worker thread as they play rather than
    /// held decoded in memory. A file from `assets/` is read as it plays; a
    /// source's bytes are kept in memory, still encoded. Loading the same
    /// name again returns the same handle. Streams are never hot reloaded.
    /// In a browser the sound is decoded whole when loaded.
    ///
    /// # Panics
    /// If the asset is missing or its header cannot be decoded (with audio
    /// enabled); [`try_load_stream`](Self::try_load_stream) returns the error.
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let music = ctx.audio.load_stream("music.ogg");
    /// ctx.audio.play_ex(&music, SoundParams { looping: true, ..Default::default() });
    /// # }
    /// ```
    pub fn load_stream(&mut self, path: &str) -> Sound {
        self.try_load_stream(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// [`load_stream`](Self::load_stream) with errors as values:
    /// [`ErrorKind::NotFound`](crate::ErrorKind::NotFound) if no source has
    /// it, [`Io`](crate::ErrorKind::Io) if it can't be read, or (with the
    /// `audio` feature) [`Decode`](crate::ErrorKind::Decode) if its header
    /// can't be decoded.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let music = ctx.audio.try_load_stream("music/theme.ogg");
    ///     assert_eq!(music.unwrap_err().kind(), ErrorKind::NotFound);
    /// });
    /// ```
    pub fn try_load_stream(&mut self, path: &str) -> Result<Sound, crate::Error> {
        use crate::assets::{Lookup, Origin};
        let kind = crate::ctx::AssetKind::Stream;
        let found = match self.asset_sources.lookup(path, |origin| {
            let key = (origin.clone(), kind);
            self.loaded.get(&key).map(|&i| self.cached_sounds[i as usize])
        })? {
            Lookup::Cached(_, sound) => return Ok(sound),
            Lookup::Found(found) => found,
        };
        let origin = found.origin.clone();
        let streamed = match &origin {
            Origin::File(file) => self.backend.stream(file.clone()),
            Origin::Source { .. } => {
                let bytes = found.into_bytes().map_err(|e| e.with_path(path))?;
                self.backend.stream_bytes(bytes)
            }
        };
        let (id, seconds) = streamed.map_err(|e| match &origin {
            Origin::File(file) => crate::Error::decode(path, format!("{}: {e}", file.display())),
            Origin::Source { .. } => crate::Error::decode(path, format!("{}: {e}", origin.describe(path))),
        })?;
        let sound = Sound { id, seconds };
        self.sounds_live.insert(id);
        self.loaded.insert((origin, kind), self.cached_sounds.len() as u64);
        self.cached_sounds.push(sound);
        Ok(sound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_effects_validate_nonfinite_and_out_of_range_parameters() {
        assert_eq!(
            AudioEffects {
                low_pass_hz: f32::NAN,
                reverb: f32::INFINITY
            }
            .validated(),
            AudioEffects::default()
        );
        assert_eq!(
            AudioEffects {
                low_pass_hz: -100.0,
                reverb: 2.0
            }
            .validated(),
            AudioEffects {
                low_pass_hz: 20.0,
                reverb: 1.0
            }
        );
    }
    #[test]
    fn audio_bus_lookup_reuses_names_and_reserves_default() {
        let mut buses = BusRegistry::default();
        let music = buses.named("music");
        assert_ne!(music, AudioBus::default());
        assert_eq!(music, buses.named("music"));
        assert_eq!(buses.named("sfx"), AudioBus::default());
        assert_eq!(buses.entries.len(), 2);
    }
    #[test]
    fn audio_headless_buses_and_effects_never_open_device() {
        crate::Game::new("audio", 1, 1).run_headless(0, |ctx| {
            let music = ctx.audio.bus("music");
            ctx.audio.set_bus_volume(music, 0.4);
            ctx.audio.mute_bus(music, true);
            ctx.audio.mute_bus(music, false);
            assert_eq!(ctx.audio.backend.buses.entries[music.0].volume, 0.4);
            assert!(!ctx.audio.backend.buses.entries[music.0].muted);
            let sound = ctx.audio.create(100, &[0.0; 100]);
            let voice = ctx
                .audio
                .play_on(&sound, SoundParams::default(), music, AudioEffects::default());
            ctx.audio.set_effects(
                voice,
                AudioEffects {
                    low_pass_hz: 600.0,
                    reverb: 0.5,
                },
            );
            ctx.audio.stop(voice);
        });
    }
    #[cfg(feature = "audio")]
    #[test]
    fn audio_volume_mapping_is_linear_gain_to_decibels() {
        assert_eq!(volume_db(1.0), kira::Decibels(0.0));
        assert!((volume_db(0.5).0 + 6.0206).abs() < 0.001);
        assert_eq!(volume_db(f32::NAN), kira::Decibels::SILENCE);
    }
}
