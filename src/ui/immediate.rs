//! Immediate widgets: a panel, a label and a button drawn the frame they are
//! called, for debug overlays and quick tools. Game UI that stays on screen
//! belongs in the retained tree ([`Ui`](super::Ui)).

use glam::Vec2;

use super::UiStyle;
use crate::camera::Rect;
use crate::gfx::Graphics;
use crate::input::{Input, MouseButton};
use crate::text::TextParams;

/// The immediate widgets of the screen's UI: `ctx.ui.immediate`.
///
/// Each call draws one widget this frame, in window pixels on top of the
/// world, in [`style`](Self::style). Nothing is kept between frames but the
/// rectangles drawn, so [`Ui::wants_pointer`](super::Ui::wants_pointer)
/// knows the mouse is over them. The widgets say what they use: all draw
/// through the graphics service, and a button also reads the mouse.
///
/// ```
/// # use ponko::prelude::*;
/// # fn tools(ctx: &mut Ctx) {
/// let panel = Rect::new(10.0, 10.0, 220.0, 120.0);
/// ctx.ui.immediate.panel(&mut ctx.gfx, panel);
/// let mut at = panel.min + 10.0;
/// at.y += ctx.ui.immediate.label(&mut ctx.gfx, at, "Simulation").y + 8.0;
/// if ctx.ui.immediate.button(&mut ctx.gfx, &ctx.input, Rect::new(at.x, at.y, 100.0, 32.0), "Pause") {
///     ctx.time.pause();
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct UiImmediate {
    /// Colors and sizes of the widgets.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// ctx.ui.immediate.style.text_size = 20.0;
    /// # }
    /// ```
    pub style: UiStyle,
    /// Every rectangle drawn this frame and the previous one, so
    /// `covers` can answer before this frame's widgets are drawn.
    this_frame: Vec<Rect>,
    last_frame: Vec<Rect>,
    /// Where the left mouse button last went down, for button clicks.
    press_at: Vec2,
}

impl Default for UiImmediate {
    fn default() -> Self {
        Self {
            style: UiStyle::default(),
            this_frame: Vec::new(),
            last_frame: Vec::new(),
            press_at: Vec2::splat(f32::NAN),
        }
    }
}

impl UiImmediate {
    /// Engine side: a new frame; last frame's rectangles still count.
    pub(crate) fn begin_frame(&mut self) {
        std::mem::swap(&mut self.this_frame, &mut self.last_frame);
        self.this_frame.clear();
    }

    /// Whether `point` (window pixels) is on a widget drawn this frame or
    /// the previous one.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// Game::new("t", 320, 240).run_headless(0, |ctx| {
    ///     let panel = Rect::new(10.0, 10.0, 100.0, 50.0);
    ///     assert!(!ctx.ui.immediate.covers(vec2(20.0, 20.0)));
    ///     ctx.ui.immediate.panel(&mut ctx.gfx, panel);
    ///     assert!(ctx.ui.immediate.covers(vec2(20.0, 20.0)));
    /// });
    /// ```
    #[must_use]
    pub fn covers(&self, point: Vec2) -> bool {
        self.this_frame
            .iter()
            .chain(&self.last_frame)
            .any(|r| r.contains(point))
    }

    /// A background panel at `rect` (window pixels), in the style's panel color.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// # fn sidebar(ctx: &mut Ctx) {
    /// let panel = Rect::new(10.0, 10.0, 220.0, 300.0);
    /// ctx.ui.immediate.panel(&mut ctx.gfx, panel);
    /// ctx.ui.immediate.label(&mut ctx.gfx, panel.min + 10.0, "Colony");
    /// # }
    /// ```
    pub fn panel(&mut self, gfx: &mut Graphics, rect: Rect) {
        gfx.ui_fill(rect, self.style.panel, -1.0);
        self.this_frame.push(rect);
    }

    /// Text at `pos` (window pixels) in the style's text size and color.
    /// Returns the space it took.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// # fn stats(ctx: &mut Ctx, ants: usize) {
    /// let mut at = vec2(20.0, 20.0);
    /// at.y += ctx.ui.immediate.label(&mut ctx.gfx, at, "Ants").y;
    /// ctx.ui.immediate.label(&mut ctx.gfx, at, &ants.to_string());
    /// # }
    /// ```
    pub fn label(&self, gfx: &mut Graphics, pos: Vec2, text: &str) -> Vec2 {
        let style = self.style;
        gfx.draw_text_ex(
            text,
            pos,
            TextParams {
                size: style.text_size,
                color: style.text,
                z: 1.0,
                ..Default::default()
            },
        );
        gfx.measure_text(text, style.text_size)
    }

    /// A button at `rect` (window pixels) with `label` centered on it.
    /// Returns `true` on the frame it's clicked (mouse released over it
    /// after being pressed over it), reading the mouse from `input`.
    ///
    /// ```
    /// # use ponko::prelude::*;
    /// # fn controls(ctx: &mut Ctx) {
    /// if ctx.ui.immediate.button(&mut ctx.gfx, &ctx.input, Rect::new(20.0, 20.0, 100.0, 32.0), "Pause") {
    ///     ctx.time.pause();
    /// }
    /// # }
    /// ```
    pub fn button(&mut self, gfx: &mut Graphics, input: &Input, rect: Rect, label: &str) -> bool {
        let style = self.style;
        let mouse = input.mouse_pos();
        if input.mouse_pressed(MouseButton::Left) {
            self.press_at = mouse;
        }
        let hovered = rect.contains(mouse);
        let held = hovered && input.mouse_down(MouseButton::Left);
        let clicked = hovered && input.mouse_released(MouseButton::Left) && rect.contains(self.press_at);
        let fill = if held {
            style.button_pressed
        } else if hovered {
            style.button_hovered
        } else {
            style.button
        };
        if hovered {
            gfx.ui_fill(rect.expand(1.0), style.accent, -0.5);
        }
        gfx.ui_fill(rect, fill, 0.0);
        let size = gfx.measure_text(label, style.text_size);
        let at = (rect.center() - size / 2.0).round();
        gfx.draw_text_ex(
            label,
            at,
            TextParams {
                size: style.text_size,
                color: style.text,
                z: 1.0,
                ..Default::default()
            },
        );
        self.this_frame.push(rect);
        clicked
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::input::Input;
    use crate::prelude::*;
    #[allow(unused_imports)]
    use crate::ui::{Ui, UiStyle};

    #[test]
    fn the_pointer_follows_the_mouse_between_systems() {
        fn first(ctx: &mut Ctx) {
            ctx.ui
                .immediate
                .panel(&mut ctx.gfx, Rect::new(100.0, 100.0, 50.0, 50.0));
            assert!(!ctx.ui.wants_pointer(), "the mouse is at (0, 0)");
            ctx.input.set_mouse_pos(vec2(120.0, 120.0));
            assert!(
                !ctx.ui.wants_pointer(),
                "the pointer is the mouse as the system found it"
            );
        }
        fn second(ctx: &mut Ctx) {
            assert!(ctx.ui.wants_pointer(), "the next system sees where the mouse went");
        }
        Game::new("t", 800, 600)
            .sim_system(first)
            .sim_system(second)
            .run_headless(1, |_| {});
    }

    #[test]
    #[allow(deprecated)]
    fn the_flat_spelling_reads_the_mouse_as_it_is_now() {
        Game::new("t", 800, 600).run_headless(0, |ctx| {
            ctx.ui
                .immediate
                .panel(&mut ctx.gfx, Rect::new(100.0, 100.0, 50.0, 50.0));
            ctx.input.set_mouse_pos(vec2(120.0, 120.0));
            assert!(ctx.mouse_over_ui());
            assert!(!ctx.ui.wants_pointer());
        });
    }
}
