# Sound

<div class="lang-pair">
<div>

```rust
let hit = ctx.load_sound("hit.ogg");   // OGG Vorbis, WAV, MP3 or FLAC from assets/
ctx.play_sound(&hit);

let music = ctx.load_sound("theme.ogg");
let song = ctx.audio.play_ex(&music, SoundParams { volume: 0.5, looping: true, ..Default::default() });
ctx.audio.stop(song);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sound}}
```

</div>
</div>

- `SoundParams { volume, speed, looping }`: `speed` also changes the pitch,
  and a little random variation keeps repeated effects from sounding
  mechanical.
- `ctx.audio.create(sample_rate, &samples)` makes a sound from generated
  mono samples (-1..1). The `ui` example's click is made this way:

<div class="lang-pair">
<div>

```rust
let samples: Vec<f32> = (0..4410) // 0.1 s at 44.1 kHz
    .map(|i| 0.3 * (i as f32 * 880.0 * std::f32::consts::TAU / 44100.0).sin())
    .collect();
let click = ctx.audio.create(44100, &samples);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sound_samples}}
```

</div>
</div>
- `Sound` is a cheap `Copy` handle, like `Texture` (C++: `ponko::sound`,
  with `play_sound(s, params)` for `play_sound_ex`).

The audio device opens the first time a sound plays. Headless runs, and
machines without audio, are silent but run the same code. For servers you
can drop the audio library entirely:

```toml
ponko = { version = "0.0.1", default-features = false }
```

The sound API stays; it just makes no noise. The C library has the same
switch: build `ponko_ffi` with `--no-default-features`. Linux builds with audio need
the ALSA headers (`libasound2-dev` on Debian and Ubuntu).

## Sound in 3D

With the `3d` feature, pair `d3::AudioSource` with `d3::Transform`, or play a
one-off sound at a point with `ctx.play_sound_at`. Every frame, after your
systems, the engine moves each sound to its entity and sets its volume,
panning and doppler from where the listener is. The listener follows
`ctx.camera3d`; to hear from somewhere else, such as the player in a
third-person game, call `ctx.audio.set_listener(Some(..))`.
Positions use world units; doppler assumes metres and takes velocities
from frame to frame. Set `doppler: 0.0` to turn it off, for example for
something that teleports.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/audio3d.rs:sound3d}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sound3d}}
```

</div>
</div>

Build this example with `cargo build --example audio3d --features 3d`.
Only running it opens a window and plays audio. Procedural clips already use
`create_sound`; file-backed clips use `load_sound`.

`d3::AudioSettings` controls volume, pitch (`sound.speed`), looping, minimum and
maximum distance, and logarithmic, linear or keyed rolloff. Keyed curves use
sorted `(normalized_distance, gain)` pairs in a static slice. All curves are
silent at the maximum distance and full volume inside the minimum distance.
Stereo panning follows the listener's right axis. Kira applies gain, panning
and playback-rate changes with smoothing; this is stereo positional audio,
not an HRTF simulation. Prefer mono source clips for localized effects.

`ctx.play_sound_at` starts a voice at a fixed world position. Save its handle
to stop it; removing an entity source stops its voice on the next audio update.
Headless execution never opens an audio device, and disabling `audio` keeps
the same APIs silent. No allocation occurs during updates unless a new voice
starts. `load_sound` decodes clips into memory; `load_stream` keeps long files streamed.

### Buses, low-pass and reverb

Ordinary playback uses the `sfx` bus. Create other named buses once with
`audio_bus`, save their handles, and pass one to `play_sound_on` or to
`d3::AudioSettings::bus`. Handles belong to the context that created them.
Volume is a linear gain; muting preserves the configured gain. Bus changes
apply to active voices as well as future playback.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/audio3d.rs:audio_mixer}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:audio_mixer}}
```

</div>
</div>

Each voice uses a Kira low-pass filter on its own child track. The cutoff
is clamped to 20–20,000 Hz. `set_sound_effects` changes cutoff and reverb
send while a voice is playing, including positional voices. Reverb is a
single shared, fully wet Kira send track; the send amount is linear 0–1.
The send follows voice attenuation, filtering and bus gain. Muting a bus
stops its new reverb input; existing shared reverb tails decay naturally.
Nonfinite cutoff/send values use defaults. No custom DSP is used. Starting
a voice allocates its track and decoder; changing parameters and updating
existing spatial voices does not allocate engine-side buffers each frame.

### Streaming music and ambience

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/audio3d.rs:audio_stream}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:audio_stream}}
```

</div>
</div>

`load_stream` reads the header and keeps the file's path (or, for music from
an [asset source](asset-sources.md), its still-encoded bytes in memory)
rather than decoding the entire clip. Each playback opens a separate decoder;
Kira performs buffered decoding on its worker thread. The returned `Sound`
works with 2D playback, positional playback, buses, effects and looping.
A file must remain available until playback ends. Streams are not hot
reloaded. In a browser there are no files to stream from: `load_stream`
decodes the fetched (or source) bytes whole, like `load_sound`.
`try_load_stream` returns errors instead of panicking. With audio disabled,
stream handles are silent and report zero duration. Headless playback never
creates tracks, decoder workers or an audio device.
