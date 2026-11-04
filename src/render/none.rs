//! Builds without the `window` feature have no GPU: `Renderer` has no
//! values, so `Ctx::renderer` is always `None` and these are never called.

use glam::Mat4;

use super::batch::DrawList;
use super::{FrameStatus, RenderStats, Screenshot};
use crate::color::Color;

pub(crate) enum Renderer {}

impl Renderer {
    pub fn stats(&self) -> RenderStats {
        match *self {}
    }
    pub fn take_screenshot(&mut self) -> Option<Screenshot> {
        match *self {}
    }
    pub fn display_wait_ms(&self) -> f32 {
        match *self {}
    }
    pub fn max_texture_size(&self) -> u32 {
        match *self {}
    }
    pub fn create_texture(&mut self, _width: u32, _height: u32, _rgba: &[u8]) -> u32 {
        match *self {}
    }
    pub fn update_texture(&mut self, _id: u32, _width: u32, _height: u32, _rgba: &[u8]) {
        match *self {}
    }
    pub fn set_filter(&mut self, _id: u32, _nearest: bool) {
        match *self {}
    }
    pub fn update_texture_region(&mut self, _id: u32, _at: [u32; 2], _size: [u32; 2], _rgba: &[u8]) {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn create_material(&mut self, _desc: &crate::three::GpuMaterialDesc) -> u32 {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn update_material(&mut self, _id: u32, _desc: &crate::three::GpuMaterialDesc) {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn max_texture_array_size(&self) -> (u32, u32) {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn create_texture_array(&mut self, _width: u32, _height: u32, _layers: &[&[u8]], _linear: bool) -> u32 {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn create_effect(&mut self, _name: &str, _source: &str) -> Result<u32, String> {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn replace_effect(&mut self, _id: u32, _name: &str, _source: &str) -> Result<(), String> {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn create_shader(&mut self, _name: &str, _source: &str) -> Result<u32, String> {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn replace_shader(&mut self, _id: u32, _name: &str, _source: &str) -> Result<(), String> {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn create_mesh(&mut self, _data: &crate::MeshData, _parts: &[(std::ops::Range<u32>, crate::Material)]) -> u32 {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn prepare_3d(&mut self, _scene: &crate::three::Scene3D<'_>, _collect_ms: f32) {
        match *self {}
    }
    pub fn render(
        &mut self,
        _world: &mut DrawList,
        _ui: &mut DrawList,
        _clear: Color,
        _world_view: Mat4,
        _ui_view: Mat4,
        _capture: bool,
    ) -> FrameStatus {
        match *self {}
    }
}
