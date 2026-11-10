//! Positional procedural audio. Build only in CI; running opens audio/window.
use ponko::*;

// ANCHOR: sound3d
fn setup(ctx: &mut Ctx) {
    let samples: Vec<f32> = (0..44100)
        .map(|i| (i as f32 * 220.0 * std::f32::consts::TAU / 44100.0).sin() * 0.1)
        .collect();
    let hum = ctx.audio.create(44100, &samples);
    let settings = AudioSettings3D {
        sound: SoundParams {
            volume: 0.4,
            looping: true,
            ..Default::default()
        },
        min_distance: 2.0,
        max_distance: 60.0,
        rolloff: AudioRolloff::Linear,
        ..Default::default()
    };
    ctx.world
        .spawn((Transform3D::at(vec3(4.0, 0.0, 0.0)), AudioSource3D::new(hum, settings)));
    ctx.audio
        .play_at(&hum, vec3(-4.0, 0.0, 0.0), AudioSettings3D::default());
}

fn main() {
    // The sounds follow their entities and the listener (ctx.camera3d) by themselves.
    Game::new("Spatial audio", 800, 600).run(setup);
}
// ANCHOR_END: sound3d

// ANCHOR: audio_mixer
#[allow(dead_code)]
fn buses_and_effects(ctx: &mut Ctx, sound: Sound) {
    let music = ctx.audio.bus("music"); // Save this handle; names are looked up once.
    ctx.audio.set_bus_volume(music, 0.6);
    let voice = ctx.audio.play_on(
        &sound,
        SoundParams {
            looping: true,
            ..Default::default()
        },
        music,
        AudioEffects {
            low_pass_hz: 4_000.0,
            reverb: 0.2,
        },
    );
    ctx.audio.set_effects(
        voice,
        AudioEffects {
            low_pass_hz: 800.0,
            reverb: 0.5,
        },
    );
    ctx.audio.mute_bus(music, true);
    ctx.audio.mute_bus(music, false); // Restores the configured gain.
    let settings = AudioSettings3D {
        bus: music,
        effects: AudioEffects {
            low_pass_hz: 2_000.0,
            reverb: 0.3,
        },
        ..Default::default()
    };
    ctx.audio.play_at(&sound, vec3(3.0, 0.0, 0.0), settings);
}
// ANCHOR_END: audio_mixer

// ANCHOR: audio_stream
#[allow(dead_code)]
fn streaming_music(ctx: &mut Ctx) {
    let music = ctx.audio.bus("music");
    let song = ctx.audio.load_stream("music.ogg");
    let voice = ctx.audio.play_on(
        &song,
        SoundParams {
            looping: true,
            ..Default::default()
        },
        music,
        AudioEffects::default(),
    );
    ctx.audio.stop(voice);
}
// ANCHOR_END: audio_stream
