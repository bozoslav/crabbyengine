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
    #[deprecated(note = "use `ctx.assets.add_source`")]
    pub fn add_asset_source(&mut self, source: impl crate::AssetSource) {
        self.assets.add_source(source)
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
    #[deprecated(note = "use `ctx.gfx.bake_reflection_probe(&mut ctx.world, entity)`")]
    #[cfg(feature = "3d")]
    pub fn bake_reflection_probe(&mut self, entity: Entity) -> Result<(), String> {
        self.gfx.bake_reflection_probe(&mut self.world, entity)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.clear`")]
    pub fn clear(&mut self, color: Color) {
        self.gfx.clear(color)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_compute`")]
    #[cfg(feature = "3d")]
    pub fn create_compute(&mut self, name: &str, source: &str) -> Compute {
        self.gfx.create_compute(name, source)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_effect`")]
    #[cfg(feature = "3d")]
    pub fn create_effect(&mut self, name: &str, source: &str) -> Effect {
        self.gfx.create_effect(name, source)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_gpu_buffer`")]
    #[cfg(feature = "3d")]
    pub fn create_gpu_buffer(&mut self, len: u32, data: &[crate::Vec4]) -> GpuBuffer {
        self.gfx.create_gpu_buffer(len, data)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_instance_batch`")]
    #[cfg(feature = "3d")]
    pub fn create_instance_batch(
        &mut self,
        mesh: Mesh,
        transforms: &[Mat4],
        options: InstanceBatchOptions,
    ) -> InstanceBatch {
        self.gfx.create_instance_batch(mesh, transforms, options)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_material`")]
    #[cfg(feature = "3d")]
    pub fn create_material(&mut self, desc: MaterialDesc) -> Material {
        self.gfx.create_material(desc)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_mesh`")]
    #[cfg(feature = "3d")]
    pub fn create_mesh(&mut self, data: &MeshData, material: Material) -> Mesh {
        self.gfx.create_mesh(data, material)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_render_target`")]
    #[cfg(feature = "3d")]
    pub fn create_render_target(&mut self, width: u32, height: u32) -> Texture {
        self.gfx.create_render_target(width, height)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_shader`")]
    #[cfg(feature = "3d")]
    pub fn create_shader(&mut self, name: &str, source: &str) -> Shader {
        self.gfx.create_shader(name, source)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_skinned_mesh`")]
    #[cfg(feature = "3d")]
    pub fn create_skinned_mesh(&mut self, data: &MeshData, material: Material, skeleton: &Skeleton) -> Mesh {
        self.gfx.create_skinned_mesh(data, material, skeleton)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.create`")]
    pub fn create_sound(&mut self, sample_rate: u32, samples: &[f32]) -> Sound {
        self.audio.create(sample_rate, samples)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_terrain_material`")]
    #[cfg(feature = "3d")]
    pub fn create_terrain_material(&mut self, desc: &TerrainMaterialDesc) -> Material {
        self.gfx.create_terrain_material(desc)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_terrain_tile`")]
    #[cfg(feature = "3d")]
    pub fn create_terrain_tile(&mut self, heights: &[f32], size: f32, material: Material) -> TerrainTile {
        self.gfx.create_terrain_tile(heights, size, material)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_texture`")]
    pub fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        self.gfx.create_texture(width, height, rgba)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_texture_array`")]
    #[cfg(feature = "3d")]
    pub fn create_texture_array(
        &mut self,
        width: u32,
        height: u32,
        layers: &[&[u8]],
        kind: TextureKind,
    ) -> TextureArray {
        self.gfx.create_texture_array(width, height, layers, kind)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.create_ui_panel`")]
    #[cfg(feature = "3d")]
    pub fn create_ui_panel(&mut self, width: u32, height: u32, size: Vec2) -> (UiPanel3D, crate::Model) {
        self.gfx.create_ui_panel(width, height, size)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.cursor_locked`")]
    pub fn cursor_locked(&self) -> bool {
        self.input.cursor_locked()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.decal_textures`")]
    #[cfg(feature = "3d")]
    pub fn decal_textures(&self) -> Option<TextureArray> {
        self.gfx.decal_textures()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.default_material`")]
    #[cfg(feature = "3d")]
    pub fn default_material(&mut self) -> Material {
        self.gfx.default_material()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.display_wait_ms`")]
    pub fn display_wait_ms(&self) -> f32 {
        self.perf.display_wait_ms()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.draw_circle`")]
    pub fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.gfx.draw_circle(center, radius, color)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.draw_ex`")]
    pub fn draw_ex(&mut self, texture: &Texture, pos: Vec2, params: DrawParams) {
        self.gfx.draw_ex(texture, pos, params)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.draw_line`")]
    pub fn draw_line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) {
        self.gfx.draw_line(a, b, width, color)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.draw_rect`")]
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.gfx.draw_rect(rect, color)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.draw_text_ex`")]
    pub fn draw_text_ex(&mut self, text: &str, pos: Vec2, params: TextParams) {
        self.gfx.draw_text_ex(text, pos, params)
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
    #[deprecated(note = "use `ctx.free(buffer)`")]
    #[cfg(feature = "3d")]
    pub fn free_gpu_buffer(&mut self, b: GpuBuffer) {
        self.free(b)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.free_instance_batch`")]
    #[cfg(feature = "3d")]
    pub fn free_instance_batch(&mut self, batch: InstanceBatch) {
        self.gfx.free_instance_batch(batch)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.free_mesh`")]
    #[cfg(feature = "3d")]
    pub fn free_mesh(&mut self, mesh: Mesh) {
        self.gfx.free_mesh(mesh)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.free_terrain_tile`")]
    #[cfg(feature = "3d")]
    pub fn free_terrain_tile(&mut self, tile: TerrainTile) {
        self.gfx.free_terrain_tile(tile)
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
    #[deprecated(note = "use `ctx.gfx.gpu_buffer_len`")]
    #[cfg(feature = "3d")]
    pub fn gpu_buffer_len(&self, b: GpuBuffer) -> Option<u32> {
        self.gfx.gpu_buffer_len(b)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.gpu_capabilities`")]
    pub fn gpu_capabilities(&self) -> Option<&crate::GpuCapabilities> {
        self.gfx.gpu_capabilities()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.gpu_passes`")]
    pub fn gpu_passes(&self) -> &[crate::GpuPass] {
        self.perf.gpu_passes()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.quality(&ctx.post, &ctx.lighting)`")]
    #[cfg(feature = "3d")]
    pub fn graphics_quality(&self) -> GraphicsQuality {
        self.gfx.quality(&self.post, &self.lighting)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.has_compute`")]
    #[cfg(feature = "3d")]
    pub fn has_compute(&self, compute: Compute) -> bool {
        self.gfx.has_compute(compute)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.has_effect`")]
    #[cfg(feature = "3d")]
    pub fn has_effect(&self, effect: Effect) -> bool {
        self.gfx.has_effect(effect)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.has_font`")]
    pub fn has_font(&self, font: Font) -> bool {
        self.gfx.has_font(font)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.has_material`")]
    #[cfg(feature = "3d")]
    pub fn has_material(&self, material: Material) -> bool {
        self.gfx.has_material(material)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.has_shader`")]
    #[cfg(feature = "3d")]
    pub fn has_shader(&self, shader: Shader) -> bool {
        self.gfx.has_shader(shader)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.has_texture_array`")]
    #[cfg(feature = "3d")]
    pub fn has_texture_array(&self, array: TextureArray) -> bool {
        self.gfx.has_texture_array(array)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.instance_batch_len`")]
    #[cfg(feature = "3d")]
    pub fn instance_batch_len(&self, batch: InstanceBatch) -> Option<usize> {
        self.gfx.instance_batch_len(batch)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.is_mesh_live`")]
    #[cfg(feature = "3d")]
    pub fn is_mesh_live(&self, mesh: Mesh) -> bool {
        self.gfx.is_mesh_live(mesh)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.is_paused`")]
    pub fn is_paused(&self) -> bool {
        self.time.is_paused()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.is_render_target`")]
    #[cfg(feature = "3d")]
    pub fn is_render_target(&self, texture: Texture) -> bool {
        self.gfx.is_render_target(texture)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.is_resident`")]
    pub fn is_resident(&self, kind: ResidencyKind, name: &str) -> bool {
        self.gfx.is_resident(kind, name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.load_with`")]
    pub fn load_asset_with<T>(&self, name: &str, decode: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<T, String> {
        self.assets.load_with(name, decode)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_clip`")]
    #[cfg(feature = "3d")]
    pub fn load_clip(&mut self, path: &str, name: &str) -> AnimationClip {
        self.gfx.load_clip(path, name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_clips`")]
    #[cfg(feature = "3d")]
    pub fn load_clips(&mut self, path: &str) -> Vec<AnimationClip> {
        self.gfx.load_clips(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.load_component_json::<T>(&mut ctx.world, path, entity)`")]
    pub fn load_component_json<T: 'static>(&mut self, path: &str, entity: Entity) -> Result<(), String> {
        self.assets.load_component_json::<T>(&mut self.world, path, entity)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.load_component_json_named(&mut ctx.world, path, entity, component)`")]
    pub fn load_component_json_named(&mut self, path: &str, entity: Entity, component: &str) -> Result<(), String> {
        self.assets
            .load_component_json_named(&mut self.world, path, entity, component)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_compute`")]
    #[cfg(feature = "3d")]
    pub fn load_compute(&mut self, path: &str) -> Compute {
        self.gfx.load_compute(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_effect`")]
    #[cfg(feature = "3d")]
    pub fn load_effect(&mut self, path: &str) -> Effect {
        self.gfx.load_effect(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_font`")]
    pub fn load_font(&mut self, path: &str) -> Font {
        self.gfx.load_font(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_mesh`")]
    #[cfg(feature = "3d")]
    pub fn load_mesh(&mut self, path: &str) -> Mesh {
        self.gfx.load_mesh(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_mesh_data`")]
    #[cfg(feature = "3d")]
    pub fn load_mesh_data(&self, path: &str) -> Result<MeshData, String> {
        self.gfx.load_mesh_data(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_reflection_probe(&mut ctx.world, entity, name)`")]
    #[cfg(feature = "3d")]
    pub fn load_reflection_probe(&mut self, entity: Entity, name: &str) -> Result<(), String> {
        self.gfx.load_reflection_probe(&mut self.world, entity, name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.load_resource::<R>(&mut ctx.world, path)`")]
    pub fn load_resource<R: Send + Sync + 'static + serde::de::DeserializeOwned>(&mut self, path: &str) {
        self.assets.load_resource::<R>(&mut self.world, path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.load_resource_json::<T>(&mut ctx.world, path)`")]
    pub fn load_resource_json<T: 'static>(&mut self, path: &str) -> Result<(), String> {
        self.assets.load_resource_json::<T>(&mut self.world, path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.load_resource_json_named(&mut ctx.world, path, name)`")]
    pub fn load_resource_json_named(&mut self, path: &str, name: &str) -> Result<(), String> {
        self.assets.load_resource_json_named(&mut self.world, path, name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_shader`")]
    #[cfg(feature = "3d")]
    pub fn load_shader(&mut self, path: &str) -> Shader {
        self.gfx.load_shader(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_skeleton`")]
    #[cfg(feature = "3d")]
    pub fn load_skeleton(&mut self, path: &str) -> Skeleton {
        self.gfx.load_skeleton(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.load_stream`")]
    pub fn load_stream(&mut self, path: &str) -> Sound {
        self.audio.load_stream(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.load_texture_array`")]
    #[cfg(feature = "3d")]
    pub fn load_texture_array(&mut self, paths: &[&str], kind: TextureKind) -> TextureArray {
        self.gfx.load_texture_array(paths, kind)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.loaded_materials`")]
    #[cfg(feature = "3d")]
    pub fn loaded_materials(&self, name: &str) -> Vec<(Material, MaterialDesc)> {
        self.gfx.loaded_materials(name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.measure_text`")]
    pub fn measure_text(&mut self, text: &str, size: f32) -> Vec2 {
        self.gfx.measure_text(text, size)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.measure_text_ex`")]
    pub fn measure_text_ex(&mut self, text: &str, font: Font, size: f32) -> Vec2 {
        self.gfx.measure_text_ex(text, font, size)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.mesh_costs`")]
    #[cfg(feature = "3d")]
    pub fn mesh_costs(&self) -> Vec<super::MeshCost> {
        self.gfx.mesh_costs()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.mesh_shadows`")]
    #[cfg(feature = "3d")]
    pub fn mesh_shadows(&self, mesh: Mesh) -> bool {
        self.gfx.mesh_shadows(mesh)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.models_drawn`")]
    #[cfg(feature = "3d")]
    pub fn models_drawn(&self) -> usize {
        self.gfx.models_drawn()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.morph_target`")]
    #[cfg(feature = "3d")]
    pub fn morph_target(&self, mesh: Mesh, name: &str) -> Option<usize> {
        self.gfx.morph_target(mesh, name)
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
    #[deprecated(note = "use `ctx.ui.wants_pointer()`")]
    pub fn mouse_over_ui(&self) -> bool {
        // The flat spelling had all of `Ctx`: it reads the mouse as it is now.
        self.ui.wants_pointer() || self.ui.immediate.covers(self.input.mouse_pos())
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
        // The flat method always counted the world as it is now, also for
        // entities this system spawned itself.
        let mut report = self.perf.report();
        report.entities = self.world.len();
        report
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
    #[deprecated(note = "use `ctx.assets.reloaded_files`")]
    pub fn reloaded_files(&self) -> &[String] {
        self.assets.reloaded_files()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.reloaded_textures`")]
    pub fn reloaded_textures(&self) -> &[crate::Texture] {
        self.assets.reloaded_textures()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.perf.render_stats`")]
    pub fn render_stats(&self) -> RenderStats {
        self.perf.render_stats()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.residency`")]
    pub fn residency(&self, kind: ResidencyKind) -> ResidencyStats {
        self.gfx.residency(kind)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.reset_temporal_history`")]
    #[cfg(feature = "3d")]
    pub fn reset_temporal_history(&mut self) {
        self.gfx.reset_temporal_history()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.time.resume`")]
    pub fn resume(&mut self) {
        self.time.resume()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.save_prefab(&ctx.world, entity, path)`")]
    pub fn save_prefab(&mut self, e: Entity, path: &str) -> Result<(), String> {
        self.assets.save_prefab(&self.world, e, path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.assets.save_scene(&ctx.world, path)`")]
    pub fn save_scene(&mut self, path: &str) -> Result<(), String> {
        self.assets.save_scene(&self.world, path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.save_screenshot`")]
    pub fn save_screenshot(&mut self, path: impl Into<PathBuf>) {
        self.gfx.save_screenshot(path)
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
    #[deprecated(note = "use `ctx.gfx.set_decal_textures`")]
    #[cfg(feature = "3d")]
    pub fn set_decal_textures(&mut self, array: Option<TextureArray>) {
        self.gfx.set_decal_textures(array)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.set_quality(&mut ctx.post, &mut ctx.lighting, quality)`")]
    #[cfg(feature = "3d")]
    pub fn set_graphics_quality(&mut self, quality: GraphicsQuality) {
        self.gfx.set_quality(&mut self.post, &mut self.lighting, quality)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.set_mesh_gpu_bounds`")]
    #[cfg(feature = "3d")]
    pub fn set_mesh_gpu_bounds(&mut self, mesh: super::Mesh, center: Vec3, radius: f32) {
        self.gfx.set_mesh_gpu_bounds(mesh, center, radius)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.set_mesh_gpu_buffer`")]
    #[cfg(feature = "3d")]
    pub fn set_mesh_gpu_buffer(&mut self, mesh: super::Mesh, b: GpuBuffer, count: u32) {
        self.gfx.set_mesh_gpu_buffer(mesh, b, count)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.set_mesh_shadows`")]
    #[cfg(feature = "3d")]
    pub fn set_mesh_shadows(&mut self, mesh: Mesh, casts: bool) {
        self.gfx.set_mesh_shadows(mesh, casts)
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
    #[deprecated(note = "use `ctx.gfx.set_texture_filter`")]
    pub fn set_texture_filter(&mut self, texture: &Texture, filter: TextureFilter) {
        self.gfx.set_texture_filter(texture, filter)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.ui.set_pointer_ray`")]
    #[cfg(feature = "3d")]
    pub fn set_ui_pointer_ray(&mut self, ray: Option<(crate::Vec3, crate::Vec3)>) {
        self.ui.set_pointer_ray(ray)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.shader_status`")]
    #[cfg(feature = "3d")]
    pub fn shader_status(&self) -> &[ShaderStatus] {
        self.gfx.shader_status()
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
    #[deprecated(note = "use `ctx.gfx.sprite_count`")]
    pub fn sprite_count(&self) -> usize {
        self.gfx.sprite_count()
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
    #[deprecated(note = "use `ctx.gfx.supports_decals`")]
    #[cfg(feature = "3d")]
    pub fn supports_decals(&self) -> bool {
        self.gfx.supports_decals()
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
    #[deprecated(note = "use `ctx.gfx.try_create_compute`")]
    #[cfg(feature = "3d")]
    pub fn try_create_compute(&mut self, name: &str, source: &str) -> Result<Compute, String> {
        self.gfx.try_create_compute(name, source)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_effect`")]
    #[cfg(feature = "3d")]
    pub fn try_create_effect(&mut self, name: &str, source: &str) -> Result<Effect, String> {
        self.gfx.try_create_effect(name, source)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_material`")]
    #[cfg(feature = "3d")]
    pub fn try_create_material(&mut self, desc: MaterialDesc) -> Result<Material, crate::Error> {
        self.gfx.try_create_material(desc)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_mesh`")]
    #[cfg(feature = "3d")]
    pub fn try_create_mesh(&mut self, data: &MeshData, material: Material) -> Result<Mesh, crate::Error> {
        self.gfx.try_create_mesh(data, material)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_render_target`")]
    #[cfg(feature = "3d")]
    pub fn try_create_render_target(&mut self, width: u32, height: u32) -> Result<Texture, String> {
        self.gfx.try_create_render_target(width, height)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_shader`")]
    #[cfg(feature = "3d")]
    pub fn try_create_shader(&mut self, name: &str, source: &str) -> Result<Shader, String> {
        self.gfx.try_create_shader(name, source)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_terrain_material`")]
    #[cfg(feature = "3d")]
    pub fn try_create_terrain_material(&mut self, desc: &TerrainMaterialDesc) -> Result<Material, String> {
        self.gfx.try_create_terrain_material(desc)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_texture_array`")]
    #[cfg(feature = "3d")]
    pub fn try_create_texture_array(
        &mut self,
        width: u32,
        height: u32,
        layers: &[&[u8]],
        kind: TextureKind,
    ) -> Result<TextureArray, String> {
        self.gfx.try_create_texture_array(width, height, layers, kind)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_create_ui_panel`")]
    #[cfg(feature = "3d")]
    pub fn try_create_ui_panel(
        &mut self,
        width: u32,
        height: u32,
        size: Vec2,
    ) -> Result<(UiPanel3D, crate::Model), String> {
        self.gfx.try_create_ui_panel(width, height, size)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_clip`")]
    #[cfg(feature = "3d")]
    pub fn try_load_clip(&mut self, path: &str, name: &str) -> Result<AnimationClip, String> {
        self.gfx.try_load_clip(path, name)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_clips`")]
    #[cfg(feature = "3d")]
    pub fn try_load_clips(&mut self, path: &str) -> Result<Vec<AnimationClip>, String> {
        self.gfx.try_load_clips(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_compute`")]
    #[cfg(feature = "3d")]
    pub fn try_load_compute(&mut self, path: &str) -> Result<Compute, String> {
        self.gfx.try_load_compute(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_effect`")]
    #[cfg(feature = "3d")]
    pub fn try_load_effect(&mut self, path: &str) -> Result<Effect, String> {
        self.gfx.try_load_effect(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_font`")]
    pub fn try_load_font(&mut self, path: &str) -> Result<Font, crate::Error> {
        self.gfx.try_load_font(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_mesh`")]
    #[cfg(feature = "3d")]
    pub fn try_load_mesh(&mut self, path: &str) -> Result<Mesh, crate::Error> {
        self.gfx.try_load_mesh(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_shader`")]
    #[cfg(feature = "3d")]
    pub fn try_load_shader(&mut self, path: &str) -> Result<Shader, String> {
        self.gfx.try_load_shader(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_skeleton`")]
    #[cfg(feature = "3d")]
    pub fn try_load_skeleton(&mut self, path: &str) -> Result<Skeleton, crate::Error> {
        self.gfx.try_load_skeleton(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.audio.try_load`")]
    pub fn try_load_sound(&mut self, path: &str) -> Result<Sound, crate::Error> {
        self.audio.try_load(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_texture`")]
    pub fn try_load_texture(&mut self, path: &str) -> Result<Texture, crate::Error> {
        self.gfx.try_load_texture(path)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_load_texture_array`")]
    #[cfg(feature = "3d")]
    pub fn try_load_texture_array(&mut self, paths: &[&str], kind: TextureKind) -> Result<TextureArray, String> {
        self.gfx.try_load_texture_array(paths, kind)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.try_set_decal_textures`")]
    #[cfg(feature = "3d")]
    pub fn try_set_decal_textures(&mut self, array: Option<TextureArray>) -> Result<(), String> {
        self.gfx.try_set_decal_textures(array)
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
    #[deprecated(note = "use `ctx.ui.immediate.button(&mut ctx.gfx, &ctx.input, rect, label)`")]
    pub fn ui_button(&mut self, rect: Rect, label: &str) -> bool {
        self.ui.immediate.button(&mut self.gfx, &self.input, rect, label)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.input.insert_text`")]
    pub fn ui_insert_text(&mut self, text: &str) {
        self.input.insert_text(text)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.ui.immediate.label(&mut ctx.gfx, pos, text)`")]
    pub fn ui_label(&mut self, pos: Vec2, text: &str) -> Vec2 {
        self.ui.immediate.label(&mut self.gfx, pos, text)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.ui.immediate.panel(&mut ctx.gfx, rect)`")]
    pub fn ui_panel(&mut self, rect: Rect) {
        self.ui.immediate.panel(&mut self.gfx, rect)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.ui.wants_keyboard`")]
    pub fn ui_wants_keyboard(&self) -> bool {
        self.ui.wants_keyboard()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.update_instance_batch`")]
    #[cfg(feature = "3d")]
    pub fn update_instance_batch(&mut self, batch: InstanceBatch, transforms: &[Mat4]) -> bool {
        self.gfx.update_instance_batch(batch, transforms)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.update_material`")]
    #[cfg(feature = "3d")]
    pub fn update_material(&mut self, material: Material, desc: MaterialDesc) {
        self.gfx.update_material(material, desc)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.update_texture`")]
    #[track_caller]
    pub fn update_texture(&mut self, texture: &Texture, rgba: &[u8]) {
        self.gfx.update_texture(texture, rgba)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.upload_terrain_tile`")]
    #[cfg(feature = "3d")]
    pub fn upload_terrain_tile(&mut self, data: &TerrainTileData, material: Material) -> TerrainTile {
        self.gfx.upload_terrain_tile(data, material)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.warm_render_pipelines`")]
    #[cfg(feature = "3d")]
    pub fn warm_render_pipelines(&mut self) {
        self.gfx.warm_render_pipelines()
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.write_animated_morphs(&mut ctx.world)`")]
    #[cfg(feature = "3d")]
    pub fn write_animated_morphs(&mut self) {
        self.gfx.write_animated_morphs(&mut self.world)
    }

    #[doc(hidden)]
    #[deprecated(note = "use `ctx.gfx.write_gpu_buffer`")]
    #[cfg(feature = "3d")]
    pub fn write_gpu_buffer(&mut self, b: GpuBuffer, offset: u32, data: &[crate::Vec4]) {
        self.gfx.write_gpu_buffer(b, offset, data)
    }
}
