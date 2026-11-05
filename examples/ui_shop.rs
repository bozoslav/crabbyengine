//! A retained shop: item grid, scrolling, input and modal confirmation.
use crabbyengine::*;
use ui_layout::prelude::*;

fn fixed(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: length(width),
            height: length(height),
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}
fn main() {
    Game::new("Shop", 900, 700).run(|ctx| {
        // ANCHOR: shop_tree
        let ui = &mut ctx.ui_retained;
        let root = ui.add(
            UiWidget::Panel,
            Style {
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                flex_direction: FlexDirection::Column,
                padding: ui_layout::Rect::length(24.0),
                gap: Size::length(12.0),
                ..Default::default()
            },
        );
        let title = ui.add_child(root, UiWidget::Label, fixed(400.0, 32.0));
        ui.set_text(title, "General store — choose an item");
        let search = ui.add_child(root, UiWidget::TextInput, fixed(360.0, 36.0));
        ui.set_text(search, "Your order note");
        let scroll = ui.add_child(
            root,
            UiWidget::Scroll,
            Style {
                size: Size {
                    width: percent(1.0),
                    height: length(360.0),
                },
                min_size: Size::length(0.0),
                flex_shrink: 1.0,
                ..Default::default()
            },
        );
        let grid = ui.add_child(
            scroll,
            UiWidget::Panel,
            Style {
                display: Display::Grid,
                size: Size {
                    width: percent(0.95),
                    height: auto(),
                },
                grid_template_columns: vec![fr(1.0), fr(1.0), fr(1.0)],
                grid_auto_rows: vec![length(76.0)],
                gap: Size::length(8.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
        );
        let mut items = Vec::new();
        for i in 0..36 {
            let item = ui.add_child(
                grid,
                UiWidget::Button,
                Style {
                    min_size: Size::length(0.0),
                    padding: ui_layout::Rect::length(8.0),
                    ..Default::default()
                },
            );
            ui.set_text(item, format!("Item {}\n{} coins", i + 1, (i + 1) * 3));
            ui.set_paint(
                item,
                UiPaint {
                    wrap: true,
                    border_width: 1.0,
                    ..Default::default()
                },
            );
            items.push(item);
        }
        let gift = ui.add_child(root, UiWidget::Toggle, fixed(220.0, 28.0));
        ui.set_text(gift, "Gift wrap");
        let buy = ui.add_child(root, UiWidget::Button, fixed(220.0, 40.0));
        ui.set_text(buy, "Buy selected item");
        ui.set_enabled(buy, false);
        ui.push_screen(root, false);
        // ANCHOR_END: shop_tree

        // ANCHOR: shop_dialog
        // A detached screen can be reused after it is popped.
        let dialog = ui.add(
            UiWidget::Panel,
            Style {
                size: Size {
                    width: length(360.0),
                    height: length(180.0),
                },
                margin: ui_layout::Rect {
                    left: auto(),
                    right: auto(),
                    top: length(180.0),
                    bottom: auto(),
                },
                padding: ui_layout::Rect::length(20.0),
                flex_direction: FlexDirection::Column,
                gap: Size::length(12.0),
                ..Default::default()
            },
        );
        let message = ui.add_child(dialog, UiWidget::Label, fixed(320.0, 40.0));
        let confirm = ui.add_child(dialog, UiWidget::Button, fixed(240.0, 36.0));
        ui.set_text(confirm, "Confirm purchase");
        let cancel = ui.add_child(dialog, UiWidget::Button, fixed(240.0, 30.0));
        ui.set_text(cancel, "Back");
        // ANCHOR_END: shop_dialog

        let mut selected = None;
        // ANCHOR: shop_events
        move |ctx: &mut Ctx| {
            // UI input/layout ran before this closure. Drain first, then mutate.
            let mut clicked = None;
            for event in ctx.ui_retained.events() {
                if let UiEvent::Clicked(id) = event {
                    clicked = Some(id);
                }
            }
            if let Some(id) = clicked {
                if let Some(index) = items.iter().position(|&item| item == id) {
                    selected = Some(index);
                    ctx.ui_retained.set_enabled(buy, true);
                    ctx.ui_retained.set_text(buy, format!("Buy item {}", index + 1));
                } else if id == buy {
                    ctx.ui_retained
                        .set_text(message, format!("Buy item {}?", selected.unwrap() + 1));
                    ctx.ui_retained.push_screen(dialog, true);
                } else if id == confirm || id == cancel {
                    ctx.ui_retained.pop_screen();
                    if id == confirm {
                        ctx.ui_retained.set_text(title, "Purchase complete!");
                    }
                }
            }
            // Tab/arrows or d-pad navigate; Enter/South confirms; Escape/East
            // pops the top screen. A modal screen also owns the outside pointer.
            if !ctx.mouse_over_ui() { /* world pointer actions */ }
            if !ctx.ui_wants_keyboard() { /* world keyboard/gamepad actions */ }
        }
        // ANCHOR_END: shop_events
    });
}
