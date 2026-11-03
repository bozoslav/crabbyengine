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
    pub fn create_material(&mut self, _uniform: &crate::three::MaterialUniform, _texture: Option<u32>) -> u32 {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn create_mesh(&mut self, _data: &crate::MeshData, _parts: &[(std::ops::Range<u32>, crate::Material)]) -> u32 {
        match *self {}
    }
    #[cfg(feature = "3d")]
    pub fn prepare_3d(
        &mut self,
        _frame: &crate::three::Frame3D,
        _lighting: &crate::Lighting,
        _camera: glam::Vec3,
        _post: &crate::Post,
        _collect_ms: f32,
    ) {
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
