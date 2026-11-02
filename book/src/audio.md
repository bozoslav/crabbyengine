# Sound

<div class="lang-pair">
<div>

```rust
let hit = ctx.load_sound("hit.ogg");   // OGG Vorbis, WAV, MP3 or FLAC from assets/
ctx.play_sound(&hit);

let music = ctx.load_sound("theme.ogg");
let song = ctx.play_sound_ex(&music, SoundParams { volume: 0.5, looping: true, ..Default::default() });
ctx.stop_sound(song);
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
- `ctx.create_sound(sample_rate, &samples)` makes a sound from generated
  mono samples (-1..1). The `ui` example's click is made this way:

<div class="lang-pair">
<div>

```rust
let samples: Vec<f32> = (0..4410) // 0.1 s at 44.1 kHz
    .map(|i| 0.3 * (i as f32 * 880.0 * std::f32::consts::TAU / 44100.0).sin())
    .collect();
let click = ctx.create_sound(44100, &samples);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sound_samples}}
```

</div>
</div>
- `Sound` is a cheap `Copy` handle, like `Texture` (C++: `crabby::sound`,
  with `play_sound(s, params)` for `play_sound_ex`).

The audio device opens the first time a sound plays. Headless runs, and
machines without audio, are silent but run the same code. For servers you
can drop the audio library entirely:

```toml
crabbyengine = { version = "...", default-features = false }
```

The sound API stays; it just makes no noise. The C library has the same
switch: build `crabby_ffi` with `--no-default-features`. Linux builds with audio need
the ALSA headers (`libasound2-dev` on Debian and Ubuntu).
