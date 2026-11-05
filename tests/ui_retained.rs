use crabbyengine::ui_layout::prelude::length;
use crabbyengine::ui_layout::{Dimension, Display, FlexDirection, LengthPercentage, Rect as Edges, Size, Style};
use crabbyengine::*;

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
            assert_eq!(ctx.typed_text(), if frame == 0 { "é猫" } else { "" });
        })
        .run_headless(2, |ctx| {
            ctx.ui_insert_text("é猫");
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
