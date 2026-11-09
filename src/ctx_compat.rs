//! The flat `Ctx` methods from before the services (`ctx.time`, `ctx.input`,
//! `ctx.gfx`, ...). Each forwards to its service and is deprecated; they are
//! removed in the next major release.
//!
//! Generated from the service signatures: keep it mechanical. A method that
//! needs more than forwarding belongs in its service.

#![allow(clippy::too_many_arguments, clippy::missing_errors_doc, clippy::missing_panics_doc)]

#[allow(unused_imports)]
use std::path::{Path, PathBuf};

use crate::*;

impl Ctx {
    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_axis`")]
    pub fn action_axis(&self, action: impl crate::ActionRef) -> f32 {
        self.input.action_axis(action)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_axis2`")]
    pub fn action_axis2(&self, action: impl crate::ActionRef) -> Vec2 {
        self.input.action_axis2(action)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_down`")]
    pub fn action_down(&self, action: impl crate::ActionRef) -> bool {
        self.input.action_down(action)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_id`")]
    pub fn action_id(&self, name: &str) -> crate::ActionId {
        self.input.action_id(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_ids`")]
    pub fn action_ids<const N: usize>(&self, names: &[&str; N]) -> [crate::ActionId; N] {
        self.input.action_ids(names)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_pressed`")]
    pub fn action_pressed(&self, action: impl crate::ActionRef) -> bool {
        self.input.action_pressed(action)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.action_released`")]
    pub fn action_released(&self, action: impl crate::ActionRef) -> bool {
        self.input.action_released(action)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.actions`")]
    pub fn actions(&self) -> &crate::ActionMap {
        self.input.actions()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.bus`")]
    pub fn audio_bus(&mut self, name: &str) -> AudioBus {
        self.audio.bus(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.listener`")]
    #[cfg(feature = "3d")]
    pub fn audio_listener(&self) -> AudioListener3D {
        self.audio.custom_listener().unwrap_or(AudioListener3D {
            position: self.camera3d.position,
            rotation: self.camera3d.rotation,
        })
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.create`")]
    pub fn create_sound(&mut self, sample_rate: u32, samples: &[f32]) -> Sound {
        self.audio.create(sample_rate, samples)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.cursor_locked`")]
    pub fn cursor_locked(&self) -> bool {
        self.input.cursor_locked()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.display_wait_ms`")]
    pub fn display_wait_ms(&self) -> f32 {
        self.perf.display_wait_ms()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.feed`")]
    pub fn feed_input(&mut self, event: crate::InputEvent) {
        self.input.feed(event)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.fixed_alpha`")]
    pub fn fixed_alpha(&self) -> f32 {
        self.time.fixed_alpha()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.fixed_dt`")]
    pub fn fixed_dt(&self) -> f32 {
        self.time.fixed_dt()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.fixed_steps`")]
    pub fn fixed_steps(&self) -> u32 {
        self.time.fixed_steps()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.fps`")]
    pub fn fps(&self) -> f32 {
        self.time.fps()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.frame`")]
    pub fn frame(&self) -> u64 {
        self.time.frame()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.frame_stats`")]
    pub fn frame_stats(&self) -> crate::FrameStats {
        self.perf.frame_stats()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.gamepad_axis`")]
    pub fn gamepad_axis(&self, pad: Gamepad, axis: GamepadAxis) -> f32 {
        self.input.gamepad_axis(pad, axis)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.gamepad_down`")]
    pub fn gamepad_down(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.input.gamepad_down(pad, button)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.gamepad_pressed`")]
    pub fn gamepad_pressed(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.input.gamepad_pressed(pad, button)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.gamepad_released`")]
    pub fn gamepad_released(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.input.gamepad_released(pad, button)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.gamepad_stick`")]
    pub fn gamepad_stick(&self, pad: Gamepad, stick: GamepadStick) -> Vec2 {
        self.input.gamepad_stick(pad, stick)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.gamepads`")]
    pub fn gamepads(&self) -> &[Gamepad] {
        self.input.gamepads()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.gpu_passes`")]
    pub fn gpu_passes(&self) -> &[crate::GpuPass] {
        self.perf.gpu_passes()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.is_paused`")]
    pub fn is_paused(&self) -> bool {
        self.time.is_paused()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.load_stream`")]
    pub fn load_stream(&mut self, path: &str) -> Sound {
        self.audio.load_stream(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.mouse_delta`")]
    pub fn mouse_delta(&self) -> Vec2 {
        self.input.mouse_delta()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.mouse_down`")]
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.input.mouse_down(button)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.mouse_motion`")]
    pub fn mouse_motion(&self) -> Vec2 {
        self.input.mouse_motion()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.mouse_pressed`")]
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.input.mouse_pressed(button)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.mouse_released`")]
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.input.mouse_released(button)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.mouse_wheel`")]
    pub fn mouse_wheel(&self) -> Vec2 {
        self.input.mouse_wheel()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.mute_bus`")]
    pub fn mute_audio_bus(&mut self, bus: AudioBus, muted: bool) {
        self.audio.mute_bus(bus, muted)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.pause`")]
    pub fn pause(&mut self) {
        self.time.pause()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.hud`")]
    pub fn perf_hud(&self) -> bool {
        self.perf.hud()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.report`")]
    pub fn perf_report(&self) -> crate::PerfReport {
        self.perf.report()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.play_at`")]
    #[cfg(feature = "3d")]
    pub fn play_sound_at(&mut self, sound: &Sound, position: Vec3, settings: AudioSettings3D) -> Playing {
        self.sync_audio_camera();
        self.audio.play_at(sound, position, settings)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.play_ex`")]
    pub fn play_sound_ex(&mut self, sound: &Sound, params: SoundParams) -> Playing {
        self.audio.play_ex(sound, params)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.play_on`")]
    pub fn play_sound_on(
        &mut self,
        sound: &Sound,
        params: SoundParams,
        bus: AudioBus,
        effects: AudioEffects,
    ) -> Playing {
        self.audio.play_on(sound, params, bus, effects)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `let _timing = ctx.perf.scope(name);`")]
    pub fn profile<R>(&mut self, name: &str, f: impl FnOnce(&mut Ctx) -> R) -> R {
        let _timing = self.perf.scope(name);
        f(self)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.profiling`")]
    pub fn profiling(&self) -> bool {
        self.perf.profiling()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.record_trace`")]
    pub fn record_trace(&mut self, path: impl Into<PathBuf>, frames: usize) {
        self.perf.record_trace(path, frames)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.render_stats`")]
    pub fn render_stats(&self) -> RenderStats {
        self.perf.render_stats()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.resume`")]
    pub fn resume(&mut self) {
        self.time.resume()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.scope_timings`")]
    pub fn scope_timings(&self) -> &[crate::ScopeTiming] {
        self.perf.scope_timings()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.set_actions`")]
    pub fn set_actions(&mut self, actions: crate::ActionMap) {
        self.input.set_actions(actions)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.set_bus_volume`")]
    pub fn set_audio_bus_volume(&mut self, bus: AudioBus, volume: f32) {
        self.audio.set_bus_volume(bus, volume)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.set_listener`")]
    #[cfg(feature = "3d")]
    pub fn set_audio_listener(&mut self, listener: Option<AudioListener3D>) {
        self.audio.set_listener(listener)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.set_cursor_locked`")]
    pub fn set_cursor_locked(&mut self, locked: bool) {
        self.input.set_cursor_locked(locked)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.set_hud`")]
    pub fn set_perf_hud(&mut self, on: bool) {
        self.perf.set_hud(on)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.set_profiling`")]
    pub fn set_profiling(&mut self, on: bool) {
        self.perf.set_profiling(on)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.set_speed`")]
    #[track_caller]
    pub fn set_sim_speed(&mut self, speed: f32) {
        self.time.set_speed(speed)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.set_effects`")]
    pub fn set_sound_effects(&mut self, playing: Playing, effects: AudioEffects) {
        self.audio.set_effects(playing, effects)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.speed`")]
    pub fn sim_speed(&self) -> f32 {
        self.time.speed()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.sim_tick`")]
    pub fn sim_tick(&self) -> u64 {
        self.time.sim_tick()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.sim_time`")]
    pub fn sim_time(&self) -> f64 {
        self.time.sim_time()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.step`")]
    pub fn step(&mut self) {
        self.time.step()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.stop`")]
    pub fn stop_sound(&mut self, playing: Playing) {
        self.audio.stop(playing)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.system_timings`")]
    pub fn system_timings(&self) -> &[SystemTiming] {
        self.perf.system_timings()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.elapsed`")]
    pub fn time(&self) -> f64 {
        self.time.elapsed()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.try_action_id`")]
    pub fn try_action_id(&self, name: &str) -> Result<crate::ActionId, crate::UnknownAction> {
        self.input.try_action_id(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.try_load`")]
    pub fn try_load_sound(&mut self, path: &str) -> Result<Sound, crate::Error> {
        self.audio.try_load(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.turns`")]
    #[track_caller]
    pub fn turns(&self, every: u32) -> Turns {
        self.time.turns(every)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.typed_text`")]
    pub fn typed_text(&self) -> &str {
        self.input.typed_text()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.insert_text`")]
    pub fn ui_insert_text(&mut self, text: &str) {
        self.input.insert_text(text)
    }
}
