use ponko::ui_layout::prelude::length;
use ponko::ui_layout::{Dimension, Display, FlexDirection, LengthPercentage, Rect as Edges, Size, Style};
use ponko::*;

fn sized(w: f32, h: f32) -> Style {
    Style {
        size: Size {
            width: Dimension::length(w),
            height: Dimension::length(h),
        },
        flex_shrink: 0.0,
        ..Default::default()
    }
}
fn scene() -> (Ui, UiId) {
    let mut ui = Ui::default();
    let root = ui.add(UiWidget::Panel, sized(300.0, 200.0));
    ui.push_screen(root, false);
    (ui, root)
}
#[test]
fn row_column_padding_and_gap_have_exact_positions() {
    let (mut ui, root) = scene();
    let mut style = sized(300.0, 200.0);
    style.flex_direction = FlexDirection::Row;
    style.padding = Edges::length(10.0);
    style.gap = Size::length(8.0);
    ui.set_layout(root, style);
    let a = ui.add_child(root, UiWidget::Button, sized(40.0, 30.0));
    let b = ui.add_child(root, UiWidget::Button, sized(50.0, 30.0));
    ui.layout(vec2(800.0, 600.0));
    assert_eq!(ui.rect(a).unwrap().min, vec2(10.0, 10.0));
    assert_eq!(ui.rect(b).unwrap().min, vec2(58.0, 10.0));
    let mut style = sized(300.0, 200.0);
    style.flex_direction = FlexDirection::Column;
    ui.set_layout(root, style);
    ui.layout(vec2(800.0, 600.0));
    assert_eq!(ui.rect(b).unwrap().min.y, 30.0);
}
#[test]
fn grid_places_inventory_cells_and_scroll_clamps() {
    let (mut ui, root) = scene();
    let scroll = ui.add_child(root, UiWidget::Scroll, sized(150.0, 80.0));
    let mut grid = sized(140.0, 200.0);
    grid.display = Display::Grid;
    grid.grid_template_columns = vec![length(60.0), length(60.0)];
    grid.gap = Size {
        width: LengthPercentage::length(4.0),
        height: LengthPercentage::length(4.0),
    };
    let grid = ui.add_child(scroll, UiWidget::Panel, grid);
    let a = ui.add_child(grid, UiWidget::Button, sized(60.0, 40.0));
    let b = ui.add_child(grid, UiWidget::Button, sized(60.0, 40.0));
    ui.layout(vec2(800.0, 600.0));
    assert_eq!(ui.rect(b).unwrap().min.x - ui.rect(a).unwrap().min.x, 64.0);
    ui.set_scroll(scroll, vec2(0.0, 999.0));
    ui.layout(vec2(800.0, 600.0));
    assert_eq!(ui.scroll(scroll), Some(vec2(0.0, 120.0)));
    assert_ne!(ui.hit_test(vec2(10.0, 100.0)), Some(a));
}
#[test]
fn press_release_events_and_disabled_focus() {
    let (mut ui, root) = scene();
    let a = ui.add_child(root, UiWidget::Button, sized(70.0, 30.0));
    let b = ui.add_child(root, UiWidget::Button, sized(70.0, 30.0));
    ui.set_enabled(a, false);
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(b));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            confirm: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.events().collect::<Vec<_>>(), vec![UiEvent::Clicked(b)]);
    assert_eq!(ui.events().count(), 0);
    let p = ui.rect(b).unwrap().center();
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: p,
            pressed: true,
            held: true,
            ..Default::default()
        },
    );
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: p,
            released: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.events().collect::<Vec<_>>(), vec![UiEvent::Clicked(b)]);
}
#[test]
fn only_top_screen_receives_input_and_cancel_restores_focus() {
    let (mut ui, root) = scene();
    let buy = ui.add_child(root, UiWidget::Button, sized(70.0, 30.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            ..Default::default()
        },
    );
    let dialog = ui.add(UiWidget::Panel, sized(300.0, 200.0));
    let yes = ui.add_child(dialog, UiWidget::Button, sized(70.0, 30.0));
    ui.push_screen(dialog, true);
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            confirm: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(yes));
    assert_eq!(ui.events().collect::<Vec<_>>(), vec![UiEvent::Clicked(yes)]);
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            cancel: true,
            confirm: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(buy));
    assert_eq!(ui.events().collect::<Vec<_>>(), vec![UiEvent::ScreenPopped(dialog)]);
}
#[test]
fn unicode_text_selection_editing_and_submission() {
    let (mut ui, root) = scene();
    let field = ui.add_child(root, UiWidget::TextInput, sized(160.0, 30.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            text: "é猫".into(),
            ..Default::default()
        },
    );
    assert_eq!(ui.text(field), Some("é猫"));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            backspace: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.text(field), Some("é"));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            select_all: true,
            text: "ok\n".into(),
            confirm: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.text(field), Some("ok"));
    assert!(ui.events().any(|e| e == UiEvent::Submitted(field)));
    assert!(ui.wants_keyboard());
}

#[test]
fn pointer_capture_survives_drag_and_scrollbar_matches_scroll_range() {
    let (mut ui, root) = scene();
    let scroll = ui.add_child(root, UiWidget::Scroll, sized(150.0, 80.0));
    ui.add_child(scroll, UiWidget::Panel, sized(140.0, 300.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(145.0, 10.0),
            pressed: true,
            held: true,
            ..Default::default()
        },
    );
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(500.0, 500.0),
            held: true,
            ..Default::default()
        },
    );
    assert!(ui.wants_pointer());
    assert_eq!(ui.scroll(scroll).unwrap().y, 220.0);
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(500.0, 500.0),
            released: true,
            ..Default::default()
        },
    );
    assert!(!ui.wants_pointer());
}
#[test]
fn navigation_reveals_clipped_buttons_and_skips_disabled_subtrees() {
    let (mut ui, root) = scene();
    let scroll = ui.add_child(root, UiWidget::Scroll, sized(100.0, 50.0));
    let mut column = sized(100.0, 150.0);
    column.flex_direction = FlexDirection::Column;
    let column = ui.add_child(scroll, UiWidget::Panel, column);
    let a = ui.add_child(column, UiWidget::Button, sized(100.0, 50.0));
    let b = ui.add_child(column, UiWidget::Button, sized(100.0, 50.0));
    let c = ui.add_child(column, UiWidget::Button, sized(100.0, 50.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(a));
    ui.set_enabled(b, false);
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            direction: vec2(0.0, 1.0),
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(c));
    assert_eq!(ui.scroll(scroll).unwrap().y, 100.0);
    assert_eq!(ui.hit_test(vec2(30.0, 20.0)), Some(c));
}
#[test]
fn stale_ids_scaling_and_absolute_constraints() {
    let (mut ui, root) = scene();
    let mut style = sized(20.0, 30.0);
    style.position = ui_layout::Position::Absolute;
    style.inset.left = ui_layout::LengthPercentageAuto::length(12.0);
    style.inset.top = ui_layout::LengthPercentageAuto::length(7.0);
    style.min_size.width = ui_layout::LengthPercentageAuto::length(40.0);
    let id = ui.add_child(root, UiWidget::Button, style);
    ui.set_scale(2.0);
    ui.layout(vec2(800.0, 600.0));
    assert_eq!(ui.rect(id).unwrap().min, vec2(24.0, 14.0));
    assert_eq!(ui.rect(id).unwrap().size(), vec2(80.0, 60.0));
    ui.remove(id);
    let new = ui.add_child(root, UiWidget::Button, sized(20.0, 20.0));
    assert_ne!(id, new);
    assert!(ui.rect(id).is_none());
}
#[test]
fn toggle_slider_wheel_and_no_release_without_press() {
    let (mut ui, root) = scene();
    let toggle = ui.add_child(root, UiWidget::Toggle, sized(70.0, 30.0));
    let slider = ui.add_child(root, UiWidget::Slider, sized(100.0, 30.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            confirm: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.value(toggle), Some(1.0));
    assert_eq!(ui.events().collect::<Vec<_>>(), vec![UiEvent::Changed(toggle)]);
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            direction: vec2(1.0, 0.0),
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(slider));
    assert_eq!(ui.value(slider), Some(0.05));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(10.0, 10.0),
            released: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.events().count(), 0);
}

#[test]
fn native_text_storage_is_shared_and_cleared_with_frame_input() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let seen = Arc::new(AtomicUsize::new(0));
    let count = seen.clone();
    Game::new("input test", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            let frame = count.fetch_add(1, Ordering::SeqCst);
            assert_eq!(ctx.input.typed_text(), if frame == 0 { "é猫" } else { "" });
        })
        .run_headless(2, |ctx| {
            ctx.input.insert_text("é猫");
        });
    assert_eq!(seen.load(Ordering::SeqCst), 2);
}

#[test]
fn focusing_text_by_pointer_does_not_submit_and_hidden_nodes_do_not_hit() {
    let (mut ui, root) = scene();
    let field = ui.add_child(root, UiWidget::TextInput, sized(120.0, 30.0));
    let hidden = ui.add_child(
        root,
        UiWidget::Button,
        Style {
            display: Display::None,
            ..sized(120.0, 30.0)
        },
    );
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(10.0, 10.0),
            pressed: true,
            held: true,
            ..Default::default()
        },
    );
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(10.0, 10.0),
            released: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(field));
    assert_eq!(ui.events().count(), 0);
    assert_ne!(ui.hit_test(Vec2::ZERO), Some(hidden));
}
#[test]
fn nested_clips_and_wheel_offsets_do_not_leak_hits() {
    let (mut ui, root) = scene();
    let outer = ui.add_child(root, UiWidget::Scroll, sized(100.0, 60.0));
    let mut column = sized(100.0, 200.0);
    column.flex_direction = FlexDirection::Column;
    let column = ui.add_child(outer, UiWidget::Panel, column);
    let first = ui.add_child(column, UiWidget::Button, sized(80.0, 40.0));
    let second = ui.add_child(column, UiWidget::Button, sized(80.0, 40.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            pointer: vec2(20.0, 20.0),
            wheel: vec2(0.0, 40.0),
            ..Default::default()
        },
    );
    assert_eq!(ui.scroll(outer), Some(vec2(0.0, 40.0)));
    assert_eq!(ui.hit_test(vec2(20.0, 20.0)), Some(second));
    assert_ne!(ui.hit_test(vec2(20.0, 80.0)), Some(first));
}

#[test]
fn last_screen_cancel_owns_input_until_next_frame_and_text_allows_vertical_navigation() {
    let (mut ui, root) = scene();
    let mut style = sized(300.0, 200.0);
    style.flex_direction = FlexDirection::Column;
    ui.set_layout(root, style);
    let field = ui.add_child(root, UiWidget::TextInput, sized(100.0, 30.0));
    let button = ui.add_child(root, UiWidget::Button, sized(100.0, 30.0));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            next: true,
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(field));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            direction: vec2(0.0, 1.0),
            ..Default::default()
        },
    );
    assert_eq!(ui.focus(), Some(button));
    ui.frame(
        vec2(800.0, 600.0),
        &UiInput {
            cancel: true,
            ..Default::default()
        },
    );
    assert!(ui.wants_keyboard());
    assert!(ui.wants_pointer());
    ui.frame(vec2(800.0, 600.0), &UiInput::default());
    assert!(!ui.wants_keyboard());
    assert!(!ui.wants_pointer());
}

// Builders, layout presets and the theme.

#[test]
fn builders_match_add_child_and_setters() {
    let mut ui = Ui::default();
    let root = ui.root().layout(UiLayout::fill()).id();
    let built = ui
        .label(root, "LOBBY")
        .layout(UiLayout::row(20.0))
        .size(13.0)
        .color(Color::YELLOW)
        .id();
    let manual = ui.add_child(root, UiWidget::Label, UiLayout::row(20.0).into_style());
    ui.set_text(manual, "LOBBY");
    let mut paint = ui.paint(manual).unwrap();
    paint.font_size = 13.0;
    paint.color = Color::YELLOW;
    ui.set_paint(manual, paint);
    assert_eq!(ui.parent(built), Some(root));
    assert_eq!(ui.text(built), ui.text(manual));
    assert_eq!(ui.layout_style(built), ui.layout_style(manual));
    let (a, b) = (ui.paint(built).unwrap(), ui.paint(manual).unwrap());
    assert_eq!(
        (a.font_size, a.color, a.background),
        (b.font_size, b.color, b.background)
    );

    let buy = ui.button(root, "Buy").enabled(false).id();
    let gift = ui.toggle(root, "Gift").value(1.0).id();
    let volume = ui.slider(root).value(0.25).id();
    let note = ui.text_input(root).text("hi\n").id();
    let list = ui.scroll_area(root).id();
    ui.push_screen(root, false);
    ui.layout(vec2(400.0, 300.0));
    assert_ne!(ui.hit_test(ui.rect(buy).unwrap().center()), Some(buy), "disabled");
    assert_eq!(ui.value(gift), Some(1.0));
    assert_eq!(ui.value(volume), Some(0.25));
    assert_eq!(ui.text(note), Some("hi"), "control characters dropped");
    assert!(ui.rect(list).is_some());
}

#[test]
fn layout_presets_place_rows_columns_anchors_and_dialogs() {
    let mut ui = Ui::default();
    let root = ui
        .root()
        .layout(UiLayout::fill().horizontal().padding(10.0).gap(5.0))
        .id();
    let side = ui.panel(root).layout(UiLayout::column(100.0)).id();
    let main = ui
        .panel(root)
        .layout(UiLayout::auto().grow(1.0).vertical().gap(4.0))
        .id();
    let header = ui.label(main, "Title").layout(UiLayout::row(30.0)).id();
    let body = ui.scroll_area(main).layout(UiLayout::fill()).id();
    let footer = ui.panel(main).layout(UiLayout::row(20.0)).id();
    let hud = ui
        .label(root, "100 HP")
        .layout(UiLayout::anchored(0.5, 0.0, 0.5, 0.1))
        .id();
    ui.push_screen(root, false);
    let dialog = ui.root().layout(UiLayout::centered(200.0, 100.0)).id();
    ui.push_screen(dialog, true);
    ui.layout(vec2(800.0, 600.0));

    let rect = |id| {
        let r = ui.rect(id).unwrap();
        (r.min, r.size())
    };
    assert_eq!(rect(side), (vec2(10.0, 10.0), vec2(100.0, 580.0)));
    assert_eq!(rect(main), (vec2(115.0, 10.0), vec2(675.0, 580.0)));
    assert_eq!(rect(header), (vec2(115.0, 10.0), vec2(675.0, 30.0)));
    // The fill shrinks to what the fixed rows leave: 580 - 30 - 20 - 2 gaps.
    assert_eq!(rect(body), (vec2(115.0, 44.0), vec2(675.0, 522.0)));
    assert_eq!(rect(footer), (vec2(115.0, 570.0), vec2(675.0, 20.0)));
    // Anchors are fractions of the parent box, out of the sibling flow.
    assert_eq!(rect(hud), (vec2(400.0, 0.0), vec2(400.0, 60.0)));
    assert_eq!(rect(dialog), (vec2(300.0, 250.0), vec2(200.0, 100.0)));

    // A different window moves anchored and centered nodes with it.
    ui.layout(vec2(1000.0, 400.0));
    assert_eq!(ui.rect(hud).unwrap().min, vec2(500.0, 0.0));
    assert_eq!(ui.rect(dialog).unwrap().min, vec2(400.0, 150.0));
}

#[test]
fn grid_preset_wraps_cells_into_equal_columns() {
    let mut ui = Ui::default();
    let root = ui.root().layout(UiLayout::fixed(320.0, 400.0)).id();
    let grid = ui.panel(root).layout(UiLayout::grid(4, 40.0).gap(0.0)).id();
    let cells: Vec<UiId> = (0..6).map(|i| ui.button(grid, format!("{i}")).id()).collect();
    ui.push_screen(root, false);
    ui.layout(vec2(800.0, 600.0));
    assert_eq!(ui.rect(cells[3]).unwrap().min, vec2(240.0, 0.0));
    assert_eq!(ui.rect(cells[4]).unwrap().min, vec2(0.0, 40.0));
    assert_eq!(ui.rect(cells[5]).unwrap().size(), vec2(80.0, 40.0));
    assert_eq!(ui.rect(grid).unwrap().size(), vec2(320.0, 80.0));
}

#[test]
fn default_theme_keeps_the_old_paint_defaults() {
    let mut ui = Ui::default();
    let root = ui.add(UiWidget::Panel, Style::default());
    let button = ui.add_child(root, UiWidget::Button, Style::default());
    let label = ui.add_child(root, UiWidget::Label, Style::default());
    let old = UiPaint::default();
    for id in [root, button] {
        let p = ui.paint(id).unwrap();
        assert_eq!(
            (p.background, p.hover, p.border, p.color, p.font_size),
            (old.background, old.hover, old.border, old.color, old.font_size)
        );
    }
    assert_eq!(ui.paint(label).unwrap().background.a, 0.0);
}

#[test]
fn theme_paints_new_nodes_and_overrides_survive_a_retheme() {
    let dark = UiTheme {
        panel: Color::BLACK,
        control: Color::rgb(0.2, 0.2, 0.2),
        accent: Color::GREEN,
        text: Color::WHITE,
        text_size: 14.0,
        border_width: 1.0,
        ..Default::default()
    };
    let mut ui = Ui::default();
    ui.set_theme(dark);
    let root = ui.root().id();
    let ok = ui.button(root, "OK").id();
    let title = ui.label(root, "Title").size(28.0).id();
    let p = ui.paint(ok).unwrap();
    assert_eq!(
        (p.background, p.border, p.border_width, p.font_size),
        (dark.control, Color::GREEN, 1.0, 14.0)
    );
    assert_eq!(ui.paint(root).unwrap().background, Color::BLACK);
    assert_eq!(ui.paint(root).unwrap().border_width, 0.0, "panels have no border");

    let light = UiTheme {
        panel: Color::WHITE,
        text: Color::BLACK,
        text_size: 16.0,
        ..dark
    };
    ui.set_theme(light);
    assert_eq!(ui.paint(root).unwrap().background, Color::WHITE);
    assert_eq!(ui.paint(ok).unwrap().color, Color::BLACK);
    let t = ui.paint(title).unwrap();
    assert_eq!(
        (t.font_size, t.color),
        (28.0, Color::BLACK),
        "own size kept, color follows"
    );
    assert_eq!(ui.theme(), &light);
}
