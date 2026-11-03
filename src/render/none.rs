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
