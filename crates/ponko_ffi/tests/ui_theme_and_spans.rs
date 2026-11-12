//! The UI theme and named grid spans through the C API.
mod common;
use common::*;
use ponko_ffi::*;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

const SCREEN: PonkoVec2 = PonkoVec2 { x: 800.0, y: 600.0 };

unsafe fn panel(ctx: *mut PonkoCtx, parent: u64, layout: &PonkoUiLayout) -> u64 {
    let mut id = PONKO_UI_NONE;
    assert_eq!(
        unsafe { ponko_ctx_ui_add_panel(ctx, PONKO_UI_SCREEN, parent, layout, &mut id) },
        PONKO_OK,
        "{}",
        last_error()
    );
    id
}
unsafe fn span_x(ctx: *mut PonkoCtx, id: u64) -> (f32, f32) {
    let mut r = PonkoRect::default();
    unsafe {
        assert_eq!(ponko_ctx_ui_layout(ctx, PONKO_UI_SCREEN, SCREEN), PONKO_OK);
        assert_eq!(ponko_ctx_ui_rect(ctx, PONKO_UI_SCREEN, id, &mut r), PONKO_OK);
    }
    (r.min.x, r.max.x)
}

#[test]
fn the_theme_reads_back_and_titles_follow_it() {
    run_ctx(|ctx| unsafe {
        let mut theme = ponko_ui_theme_default();
        assert_eq!(ponko_ctx_ui_theme(ctx, PONKO_UI_SCREEN, &mut theme), PONKO_OK);
        assert_eq!(theme, ponko_ui_theme_default(), "a new tree has the default theme");
        let root = panel(ctx, PONKO_UI_NONE, &ponko_ui_layout_default());
        let mut title = PONKO_UI_NONE;
        assert_eq!(
            ponko_ctx_ui_add_title(ctx, PONKO_UI_SCREEN, root, c"Shop".as_ptr(), null(), &mut title),
            PONKO_OK
        );
        theme.title = PonkoColor {
            r: 1.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        theme.title_size = 30.0;
        assert_eq!(ponko_ctx_ui_set_theme(ctx, PONKO_UI_SCREEN, &theme), PONKO_OK);
        let mut read = ponko_ui_theme_default();
        assert_eq!(ponko_ctx_ui_theme(ctx, PONKO_UI_SCREEN, &mut read), PONKO_OK);
        assert_eq!(read, theme);
        let mut paint = ponko_ui_paint_default();
        assert_eq!(ponko_ctx_ui_paint(ctx, PONKO_UI_SCREEN, title, &mut paint), PONKO_OK);
        assert_eq!(
            (paint.font_size, paint.color),
            (30.0, theme.title),
            "the title follows the theme"
        );

        theme.text_size = -1.0;
        assert_eq!(
            ponko_ctx_ui_set_theme(ctx, PONKO_UI_SCREEN, &theme),
            PONKO_INVALID_ARGUMENT
        );
        theme.text_size = 18.0;
        theme.font = PonkoFont { id: 999 };
        assert_eq!(
            ponko_ctx_ui_set_theme(ctx, PONKO_UI_SCREEN, &theme),
            PONKO_INVALID_ARGUMENT,
            "unknown font"
        );
        assert_eq!(ponko_ctx_ui_theme(ctx, PONKO_UI_SCREEN, &mut read), PONKO_OK);
        assert_eq!(read.title_size, 30.0, "a rejected theme changes nothing");
    });
}

#[test]
fn a_named_span_ends_at_the_nth_line_of_that_name() {
    run_ctx(|ctx| unsafe {
        // Four 100-unit columns; lines 3 and 5 are both called "edge".
        let track = PonkoUiTrack {
            kind: PonkoUiTrackKind::PONKO_UI_TRACK_POINTS as u32,
            value: 100.0,
        };
        let columns = [PonkoUiTemplateTrack {
            track,
            repetition: 0,
            count: 0,
            tracks: null(),
            track_count: 0,
            line_names: null(),
            line_name_count: 0,
        }; 4];
        let mut layout = ponko_ui_layout_default();
        layout.display = PonkoUiDisplay::PONKO_UI_DISPLAY_GRID as u32;
        layout.grid_template_columns = columns.as_ptr();
        layout.grid_template_column_count = 4;
        let root = panel(ctx, PONKO_UI_NONE, &layout);
        let names = [
            c"".as_ptr(),
            c"".as_ptr(),
            c"edge".as_ptr(),
            c"".as_ptr(),
            c"edge".as_ptr(),
        ];
        assert_eq!(
            ponko_ctx_ui_set_grid_line_names(ctx, PONKO_UI_SCREEN, root, 0, names.as_ptr(), 5),
            PONKO_OK
        );
        let mut item = ponko_ui_layout_default();
        item.grid_column_start = PonkoUiGridLine {
            kind: PonkoUiGridLineKind::PONKO_UI_GRID_LINE as u32,
            value: 1,
        };
        let child = panel(ctx, root, &item);
        assert_eq!(ponko_ctx_ui_push_screen(ctx, PONKO_UI_SCREEN, root, false), PONKO_OK);

        assert_eq!(
            ponko_ctx_ui_set_grid_named_span(ctx, PONKO_UI_SCREEN, child, 0, true, c"edge".as_ptr(), 1),
            PONKO_OK
        );
        assert_eq!(span_x(ctx, child), (0.0, 200.0), "to the first edge");
        assert_eq!(
            ponko_ctx_ui_set_grid_named_span(ctx, PONKO_UI_SCREEN, child, 0, true, c"edge".as_ptr(), 2),
            PONKO_OK
        );
        assert_eq!(span_x(ctx, child), (0.0, 400.0), "to the second edge");

        // From an end line back to a named line.
        let mut item = ponko_ui_layout_default();
        item.grid_column_end = PonkoUiGridLine {
            kind: PonkoUiGridLineKind::PONKO_UI_GRID_LINE as u32,
            value: 5,
        };
        assert_eq!(ponko_ctx_ui_set_layout(ctx, PONKO_UI_SCREEN, child, &item), PONKO_OK);
        assert_eq!(
            ponko_ctx_ui_set_grid_named_span(ctx, PONKO_UI_SCREEN, child, 0, false, c"edge".as_ptr(), 1),
            PONKO_OK
        );
        assert_eq!(
            span_x(ctx, child),
            (200.0, 400.0),
            "back to the nearest edge before line 5"
        );
    });
}

#[test]
fn bad_named_spans_are_rejected() {
    run_ctx(|ctx| unsafe {
        let child = panel(ctx, PONKO_UI_NONE, &ponko_ui_layout_default());
        let set = |axis, name: *const std::ffi::c_char, count| {
            ponko_ctx_ui_set_grid_named_span(ctx, PONKO_UI_SCREEN, child, axis, true, name, count)
        };
        assert_eq!(set(0, c"edge".as_ptr(), 0), PONKO_INVALID_ARGUMENT, "count 0");
        assert_eq!(set(0, c"edge".as_ptr(), 65536), PONKO_INVALID_ARGUMENT, "count too big");
        assert_eq!(set(2, c"edge".as_ptr(), 1), PONKO_INVALID_ARGUMENT, "axis");
        assert_eq!(
            set(0, c"1edge".as_ptr(), 1),
            PONKO_INVALID_ARGUMENT,
            "not an identifier"
        );
        assert_eq!(set(0, null(), 1), PONKO_INVALID_ARGUMENT);
        assert_eq!(set(1, c"edge".as_ptr(), 65535), PONKO_OK);
        assert_eq!(
            ponko_ctx_ui_set_grid_named_span(ctx, PONKO_UI_SCREEN, PONKO_UI_NONE, 0, true, c"edge".as_ptr(), 1),
            PONKO_NOT_FOUND
        );
        assert_eq!(
            ponko_ctx_ui_set_grid_named_span(null_mut(), PONKO_UI_SCREEN, child, 0, true, c"edge".as_ptr(), 1),
            PONKO_INVALID_ARGUMENT
        );
    });
}
