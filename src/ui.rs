//! Retained game UI, shapes and immediate-mode panels, labels and buttons.
//!
//! [`Ui`] owns a persistent node tree and queued events. Layout is delegated
//! to [Taffy](https://docs.rs/taffy/0.15.1/taffy/), which provides flexbox/grid
//! without owning rendering or an event loop. The engine's sprites and text
//! render the widgets. [Egui](https://github.com/emilk/egui) is easy to embed
//! but immediate-mode; it remains the inspector rather than the game UI.
//! [Xilem/Masonry](https://docs.rs/xilem/latest/xilem/) is retained but its
//! standard integration brings Vello rendering and winit window handling.
//! An owned tree plus Taffy keeps game textures, styling and input ownership
//! in one place while using a proven layout implementation.
//!
//! Shapes are drawn in world units like sprites. UI is drawn in window
//! pixels on top of everything. Both are built from sprites, so they batch
//! with everything else.

use glam::Vec2;

use crate::camera::Rect;
use crate::color::Color;
use crate::ctx::Ctx;
use crate::input::MouseButton;
use crate::text::TextParams;

/// Colors and sizes of the built-in UI: [`ctx.ui_style`](crate::Ctx::ui_style).
///
/// ```
/// # use ponko::*;
/// # fn setup(ctx: &mut Ctx) {
/// ctx.ui_style.accent = Color::hex(0x2e8b57);
/// ctx.ui_style.text_size = 18.0;
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiStyle {
    /// A panel's background.
    pub panel: Color,
    /// A button's background.
    pub button: Color,
    /// A button's background while the mouse is over it.
    pub button_hovered: Color,
    /// A button's background while held down.
    pub button_pressed: Color,
    /// Used for highlights, such as a button's outline while hovered.
    pub accent: Color,
    /// Text color.
    pub text: Color,
    /// Text size in pixels.
    pub text_size: f32,
    /// Space between a panel's edge and its contents, in pixels.
    pub padding: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            panel: Color::rgba(0.08, 0.09, 0.12, 0.85),
            button: Color::rgba(0.20, 0.22, 0.28, 1.0),
            button_hovered: Color::rgba(0.27, 0.30, 0.38, 1.0),
            button_pressed: Color::rgba(0.15, 0.16, 0.21, 1.0),
            accent: Color::hex(0xf28c28),
            text: Color::rgba(0.93, 0.93, 0.95, 1.0),
            text_size: 16.0,
            padding: 10.0,
        }
    }
}

/// UI state that lives across frames.
#[derive(Default)]
pub(crate) struct UiState {
    /// Every UI rectangle drawn this frame and the previous one, so
    /// `mouse_over_ui` can answer before this frame's UI is drawn.
    this_frame: Vec<Rect>,
    last_frame: Vec<Rect>,
    /// The mouse is over the inspector overlay.
    pub overlay_has_pointer: bool,
    retained_spare: Option<Ui>,
    retained_input_text: String,
    /// World-space panels' input and drawing.
    #[cfg(feature = "3d")]
    pub panels: panel3d::Panels,
}

impl UiState {
    pub fn begin_frame(&mut self) {
        std::mem::swap(&mut self.this_frame, &mut self.last_frame);
        self.this_frame.clear();
    }
}

impl Ctx {
    // ------------------------------------------------------------ shapes

    // ------------------------------------------------------------ UI

    /// `true` if the mouse is over UI drawn this frame or the previous one.
    /// Check it before reacting to clicks in the world, so clicking a
    /// button doesn't also select a unit behind it.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn select(ctx: &mut Ctx) {
    /// if ctx.input.mouse_pressed(MouseButton::Left) && !ctx.mouse_over_ui() {
    ///     let at = ctx.mouse_world();
    ///     # let _ = at;
    /// }
    /// # }
    /// ```
    pub fn mouse_over_ui(&self) -> bool {
        let m = self.mouse_pos();
        self.ui_retained.wants_pointer()
            || self.ui.overlay_has_pointer
            || self
                .ui
                .this_frame
                .iter()
                .chain(&self.ui.last_frame)
                .any(|r| r.contains(m))
    }

    /// A background panel at `rect` (window pixels), in the style's panel color.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn sidebar(ctx: &mut Ctx) {
    /// let panel = Rect::new(10.0, 10.0, 220.0, 300.0);
    /// ctx.ui_panel(panel);
    /// ctx.ui_label(panel.min + 10.0, "Colony");
    /// # }
    /// ```
    pub fn ui_panel(&mut self, rect: Rect) {
        let color = self.ui_style.panel;
        self.gfx.ui_fill(rect, color, -1.0);
        self.ui.this_frame.push(rect);
    }

    /// Text at `pos` (window pixels) in the style's text size and color.
    /// Returns the space it took.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn stats(ctx: &mut Ctx, ants: usize) {
    /// let mut at = vec2(20.0, 20.0);
    /// at.y += ctx.ui_label(at, "Ants").y;
    /// ctx.ui_label(at, &ants.to_string());
    /// # }
    /// ```
    pub fn ui_label(&mut self, pos: Vec2, text: &str) -> Vec2 {
        let style = self.ui_style;
        self.gfx.draw_text_ex(
            text,
            pos,
            TextParams {
                size: style.text_size,
                color: style.text,
                z: 1.0,
                ..Default::default()
            },
        );
        self.gfx.measure_text(text, style.text_size)
    }

    /// A button at `rect` (window pixels) with `label` centered on it.
    /// Returns `true` on the frame it's clicked (mouse released over it
    /// after being pressed over it).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn controls(ctx: &mut Ctx) {
    /// if ctx.ui_button(Rect::new(20.0, 20.0, 100.0, 32.0), "Pause") {
    ///     ctx.time.pause();
    /// }
    /// # }
    /// ```
    pub fn ui_button(&mut self, rect: Rect, label: &str) -> bool {
        let style = self.ui_style;
        let mouse = self.mouse_pos();
        if self.input.mouse_pressed(MouseButton::Left) {
            self.ui_press_at = mouse;
        }
        let hovered = rect.contains(mouse);
        let held = hovered && self.input.mouse_down(MouseButton::Left);
        let clicked = hovered && self.input.mouse_released(MouseButton::Left) && rect.contains(self.ui_press_at);
        let fill = if held {
            style.button_pressed
        } else if hovered {
            style.button_hovered
        } else {
            style.button
        };
        if hovered {
            self.gfx.ui_fill(rect.expand(1.0), style.accent, -0.5);
        }
        self.gfx.ui_fill(rect, fill, 0.0);
        let size = self.gfx.measure_text(label, style.text_size);
        let at = (rect.center() - size / 2.0).round();
        self.gfx.draw_text_ex(
            label,
            at,
            TextParams {
                size: style.text_size,
                color: style.text,
                z: 1.0,
                ..Default::default()
            },
        );
        self.ui.this_frame.push(rect);
        clicked
    }
}

mod access;
mod build;
#[cfg(feature = "3d")]
mod panel3d;
mod retained;
mod retained_draw;
pub use access::{UiAccessNode, UiAccessTree, UiAccessValue, UiRole};
pub use build::{UiBuilder, UiLayout, UiTheme};
#[cfg(feature = "3d")]
pub use panel3d::UiPanel3D;
pub use retained::{Ui, UiEvent, UiId, UiInput, UiPaint, UiTextAlign, UiWidget};
