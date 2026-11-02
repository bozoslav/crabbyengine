# Sound

```rust
let hit = ctx.load_sound("hit.ogg");   // OGG Vorbis, WAV, MP3 or FLAC from assets/
ctx.play_sound(&hit);

let music = ctx.load_sound("theme.ogg");
let song = ctx.play_sound_ex(&music, SoundParams { volume: 0.5, looping: true, ..Default::default() });
ctx.stop_sound(song);
```

- `SoundParams { volume, speed, looping }`: `speed` also changes the pitch,
  and a little random variation keeps repeated effects from sounding
  mechanical.
- `ctx.create_sound(sample_rate, &samples)` makes a sound from generated
  mono samples (-1..1). The `ui` example's click is made this way.
- `Sound` is a cheap `Copy` handle, like `Texture`.

The audio device opens the first time a sound plays. Headless runs, and
machines without audio, are silent but run the same code. For servers you
can drop the audio library entirely:

```toml
crabbyengine = { version = "...", default-features = false }
```

The sound API stays; it just makes no noise. Linux builds with audio need
the ALSA headers (`libasound2-dev` on Debian and Ubuntu).
