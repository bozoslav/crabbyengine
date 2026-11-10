use super::retained::*;
use crate::render::batch::{SpriteInstance, uv_region};
use crate::{Color, Ctx, GamepadButton, Key, MouseButton, Rect, TextParams, Vec2};

impl Ctx {
    pub(crate) fn update_retained_ui(&mut self) {
        let text = std::mem::take(&mut self.ui_state.retained_input_text);
        let input = self.retained_input(&self.ui, text);
        self.ui.frame_with_text(self.window_size(), &input, &mut self.gfx.text);
        self.ui_state.retained_input_text = input.text;
        #[cfg(feature = "3d")]
        self.update_ui_panels();
    }

    /// This frame's input for `ui`, as the window's mouse, keyboard and
    /// gamepads give it; `text` (emptied) receives the typed text.
    pub(super) fn retained_input(&self, ui: &crate::Ui, mut text: String) -> crate::UiInput {
        text.clear();
        text.push_str(self.input.typed_text());
        let pressed = |key| self.key_pressed(key);
        let pad = |b| self.input.gamepads().iter().any(|&p| self.input.gamepad_pressed(p, b));
        let shift = self.key_down(Key::LeftShift) || self.key_down(Key::RightShift);
        let ctrl = self.key_down(Key::LeftCtrl)
            || self.key_down(Key::RightCtrl)
            || self.key_down(Key::LeftSuper)
            || self.key_down(Key::RightSuper);
        crate::UiInput {
            pointer: self.mouse_pos(),
            pressed: self.input.mouse_pressed(MouseButton::Left),
            held: self.input.mouse_down(MouseButton::Left),
            released: self.input.mouse_released(MouseButton::Left),
            wheel: -self.input.mouse_wheel() * 40.0,
            next: pressed(Key::Tab) && !shift,
            previous: pressed(Key::Tab) && shift,
            direction: Vec2::new(
                (pressed(Key::Right) || pad(GamepadButton::DPadRight)) as u8 as f32
                    - (pressed(Key::Left) || pad(GamepadButton::DPadLeft)) as u8 as f32,
                (pressed(Key::Down) || pad(GamepadButton::DPadDown)) as u8 as f32
                    - (pressed(Key::Up) || pad(GamepadButton::DPadUp)) as u8 as f32,
            ),
            confirm: pressed(Key::Enter)
                || pad(GamepadButton::South)
                || (pressed(Key::Space)
                    && !ui
                        .focus()
                        .is_some_and(|id| ui.node(id).is_some_and(|n| n.widget == crate::UiWidget::TextInput))),
            cancel: pressed(Key::Escape) || pad(GamepadButton::East),
            shift,
            select_all: ctrl && pressed(Key::A),
            backspace: pressed(Key::Backspace),
            delete: pressed(Key::Delete),
            home: pressed(Key::Home),
            end: pressed(Key::End),
            text,
        }
    }

    pub(crate) fn draw_retained_ui(&mut self) {
        // Move the tree out while issuing draw calls; no callback borrowing the
        // game, context and node tree simultaneously is required.
        let spare = self.ui_state.retained_spare.take().unwrap_or_default();
        let mut ui = std::mem::replace(&mut self.ui, spare);
        self.draw_ui_tree(&mut ui, self.window_size());
        std::mem::swap(&mut self.ui, &mut ui);
        self.ui_state.retained_spare = Some(ui);
    }

    /// Draws `ui`, laid out in `viewport`, into `ui_list`.
    pub(crate) fn draw_ui_tree(&mut self, ui: &mut crate::Ui, viewport: Vec2) {
        // Mutations made by game systems (including screen pushes) show now.
        ui.layout_with_text(viewport, &mut self.gfx.text);
        let scale = ui.scale();
        for (order, &id) in ui.visible.iter().enumerate() {
            let n = ui.node(id).unwrap();
            let z = 100.0 + order as f32 * 4.0;
            if ui.modal_root(id) {
                self.gfx.ui_list.set_clip(None);
                self.gfx.ui_fill(
                    Rect::new(0.0, 0.0, viewport.x, viewport.y),
                    Color::rgba(0.0, 0.0, 0.0, 0.55),
                    z - 1.0,
                );
            }
            self.gfx.ui_list.set_clip(Some(n.clip));
            let fill = if !n.enabled {
                n.paint.disabled
            } else if ui.pressed == Some(id) {
                n.paint.pressed
            } else if ui.hovered == Some(id) || ui.focus == Some(id) {
                n.paint.hover
            } else {
                n.paint.background
            };
            let width = if ui.focus == Some(id) {
                n.paint.border_width.max(2.0)
            } else {
                n.paint.border_width
            } * scale;
            if width > 0.0 {
                self.gfx.ui_fill(n.rect, n.paint.border, z);
            }
            let rect = n.rect.expand(-width);
            self.gfx.ui_fill(rect, fill, z + 0.1);
            if let Some(texture) = n.paint.texture {
                self.retained_image(
                    texture,
                    rect,
                    n.paint.slice * scale,
                    n.paint.slice,
                    n.paint.color,
                    z + 0.2,
                );
            }
            self.gfx.ui_list.set_clip(Some(intersect(n.clip, n.rect)));
            let layout = ui.tree.layout(n.layout).unwrap();
            let inset = Vec2::new(
                layout.padding.left + layout.border.left,
                layout.padding.top + layout.border.top,
            ) * scale;
            let content = Rect {
                min: n.rect.min + inset,
                max: n.rect.max
                    - Vec2::new(
                        layout.padding.right + layout.border.right,
                        layout.padding.bottom + layout.border.bottom,
                    ) * scale,
            };
            let mut text_rect = content;
            if n.widget == crate::UiWidget::Toggle {
                let side = content.size().y.min(20.0 * scale);
                self.gfx.ui_fill(
                    Rect::new(content.min.x, content.min.y, side, side),
                    n.paint.border,
                    z + 0.4,
                );
                if n.value > 0.5 {
                    self.gfx.ui_fill(
                        Rect::new(
                            content.min.x + 4.0 * scale,
                            content.min.y + 4.0 * scale,
                            (side - 8.0 * scale).max(0.0),
                            (side - 8.0 * scale).max(0.0),
                        ),
                        n.paint.color,
                        z + 0.5,
                    );
                }
                text_rect.min.x += side + 6.0 * scale;
            }
            if n.widget == crate::UiWidget::Slider {
                self.gfx.ui_fill(
                    Rect::new(
                        content.min.x,
                        content.center().y - 2.0 * scale,
                        content.size().x,
                        4.0 * scale,
                    ),
                    n.paint.border,
                    z + 0.4,
                );
                self.gfx.ui_fill(
                    Rect::new(
                        content.min.x + n.value * (content.size().x - 8.0 * scale),
                        content.min.y,
                        8.0 * scale,
                        content.size().y,
                    ),
                    n.paint.color,
                    z + 0.5,
                );
            }
            let font_size = n.paint.font_size * scale;
            let mut origin = text_rect.min;
            if n.widget == crate::UiWidget::TextInput && ui.focus == Some(id) {
                let caret = self.gfx.measure_text_ex(&n.text[..n.caret], n.paint.font, font_size).x;
                origin.x -= (caret - text_rect.size().x + 3.0 * scale).max(0.0);
                let a = self
                    .gfx
                    .measure_text_ex(&n.text[..n.anchor.min(n.caret)], n.paint.font, font_size)
                    .x;
                let b = self
                    .gfx
                    .measure_text_ex(&n.text[..n.anchor.max(n.caret)], n.paint.font, font_size)
                    .x;
                if a != b {
                    self.gfx.ui_fill(
                        Rect::new(origin.x + a, origin.y, b - a, font_size),
                        n.paint.hover,
                        z + 0.6,
                    );
                }
                self.gfx.ui_fill(
                    Rect::new(origin.x + caret, origin.y, scale, font_size),
                    n.paint.color,
                    z + 0.8,
                );
            }
            let params = TextParams {
                font: n.paint.font,
                size: font_size,
                color: n.paint.color,
                z: z + 0.7,
            };
            self.retained_text(&n.text, origin, text_rect.size().x, params, n.paint.wrap, n.paint.align);
            if n.widget == crate::UiWidget::Scroll && n.max_scroll.y > 0.0 {
                let h = n.rect.size().y;
                let thumb = (h * h / (h + n.max_scroll.y * scale)).max(20.0 * scale).min(h);
                let y = n.rect.min.y + (h - thumb) * n.scroll.y / n.max_scroll.y;
                self.gfx.ui_fill(
                    Rect::new(n.rect.max.x - 8.0 * scale, y, 6.0 * scale, thumb),
                    n.paint.border,
                    z + 0.9,
                );
            }
        }
        self.gfx.ui_list.set_clip(None);
    }
    fn retained_text(
        &mut self,
        text: &str,
        origin: Vec2,
        width: f32,
        params: TextParams,
        wrap: bool,
        align: crate::UiTextAlign,
    ) {
        let mut y = origin.y;
        for paragraph in text.split('\n') {
            let mut rest = paragraph;
            loop {
                let end = if wrap {
                    wrap_end(&mut self.gfx.text, rest, params.font, params.size, width)
                } else {
                    rest.len()
                };
                let line = &rest[..end];
                let w = self.gfx.measure_text_ex(line, params.font, params.size).x;
                let x = origin.x
                    + match align {
                        crate::UiTextAlign::Start => 0.0,
                        crate::UiTextAlign::Center => (width - w) * 0.5,
                        crate::UiTextAlign::End => width - w,
                    };
                self.gfx.draw_text_ex(line, Vec2::new(x, y), params);
                y += self.gfx.measure_text_ex(line, params.font, params.size).y;
                if end == rest.len() {
                    break;
                }
                rest = rest[end..].trim_start();
            }
        }
    }
    fn retained_image(
        &mut self,
        texture: crate::Texture,
        rect: Rect,
        border: f32,
        source_border: f32,
        color: Color,
        z: f32,
    ) {
        let source = texture.size();
        let src = source_border.min(source.x * 0.5).min(source.y * 0.5).max(0.0);
        let bx = border.min(rect.size().x * 0.5).max(0.0);
        let by = border.min(rect.size().y * 0.5).max(0.0);
        let xs = [0.0, bx, rect.size().x - bx, rect.size().x];
        let ys = [0.0, by, rect.size().y - by, rect.size().y];
        let us = [0.0, src / source.x, 1.0 - src / source.x, 1.0];
        let vs = [0.0, src / source.y, 1.0 - src / source.y, 1.0];
        for y in 0..3 {
            for x in 0..3 {
                let size = Vec2::new(xs[x + 1] - xs[x], ys[y + 1] - ys[y]);
                if size.cmpgt(Vec2::ZERO).all() {
                    self.gfx.ui_list.push(
                        texture.id,
                        z,
                        SpriteInstance::new(
                            (rect.min + Vec2::new(xs[x], ys[y])).into(),
                            size.into(),
                            [0.0, 0.0],
                            0.0,
                            color.to_rgba8(),
                        )
                        .with_uv(uv_region([us[x], vs[y]], [us[x + 1] - us[x], vs[y + 1] - vs[y]])),
                    );
                }
            }
        }
    }
}
