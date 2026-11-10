//! A retained shop: item grid, scrolling, input and modal confirmation.
use ponko::*;

fn main() {
    Game::new("Shop", 900, 700).run(|ctx| {
        // ANCHOR: shop_tree
        let ui = &mut ctx.ui;
        let root = ui.root().layout(UiLayout::fill().padding(24.0).gap(12.0)).id();
        let title = ui
            .label(root, "General store — choose an item")
            .layout(UiLayout::fixed(400.0, 32.0))
            .id();
        ui.text_input(root)
            .text("Your order note")
            .layout(UiLayout::fixed(360.0, 36.0));
        let scroll = ui.scroll_area(root).layout(UiLayout::fill().height(360.0)).id();
        // 95% wide, so the scrollbar doesn't cover the last column.
        let grid = ui
            .panel(scroll)
            .layout(UiLayout::grid(3, 76.0).gap(8.0).width_fraction(0.95))
            .id();
        let items: Vec<UiId> = (1..=36)
            .map(|i| {
                ui.button(grid, format!("Item {i}\n{} coins", i * 3))
                    .layout(UiLayout::auto().padding(8.0))
                    .wrap()
                    .border_width(1.0)
                    .id()
            })
            .collect();
        ui.toggle(root, "Gift wrap").layout(UiLayout::fixed(220.0, 28.0));
        let buy = ui
            .button(root, "Buy selected item")
            .layout(UiLayout::fixed(220.0, 40.0))
            .enabled(false)
            .id();
        ui.push_screen(root, false);
        // ANCHOR_END: shop_tree

        // ANCHOR: shop_dialog
        // A detached screen can be reused after it is popped.
        let dialog = ui
            .root()
            .layout(UiLayout::centered(360.0, 180.0).padding(20.0).gap(12.0))
            .id();
        let message = ui.label(dialog, "").layout(UiLayout::fixed(320.0, 40.0)).id();
        let confirm = ui
            .button(dialog, "Confirm purchase")
            .layout(UiLayout::fixed(240.0, 36.0))
            .id();
        let cancel = ui.button(dialog, "Back").layout(UiLayout::fixed(240.0, 30.0)).id();
        // ANCHOR_END: shop_dialog

        let mut selected = None;
        // ANCHOR: shop_events
        move |ctx: &mut Ctx| {
            // UI input/layout ran before this closure. Drain first, then mutate.
            let mut clicked = None;
            for event in ctx.ui.events() {
                if let UiEvent::Clicked(id) = event {
                    clicked = Some(id);
                }
            }
            if let Some(id) = clicked {
                if let Some(index) = items.iter().position(|&item| item == id) {
                    selected = Some(index);
                    ctx.ui.set_enabled(buy, true);
                    ctx.ui.set_text(buy, format!("Buy item {}", index + 1));
                } else if id == buy {
                    ctx.ui.set_text(message, format!("Buy item {}?", selected.unwrap() + 1));
                    ctx.ui.push_screen(dialog, true);
                } else if id == confirm || id == cancel {
                    ctx.ui.pop_screen();
                    if id == confirm {
                        ctx.ui.set_text(title, "Purchase complete!");
                    }
                }
            }
            // Tab/arrows or d-pad navigate; Enter/South confirms; Escape/East
            // pops the top screen. A modal screen also owns the outside pointer.
            if !ctx.ui.wants_pointer() { /* world pointer actions */ }
            if !ctx.ui.wants_keyboard() { /* world keyboard/gamepad actions */ }
        }
        // ANCHOR_END: shop_events
    });
}
