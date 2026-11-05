//! Retained UI from C (Rust `Ui`, `UiWidget`, `UiPaint`, `Ctx::ui_retained`).
//!
//! Build a tree once with `crabby_ctx_ui_add_*`, give roots to
//! `crabby_ctx_ui_push_screen`, and change text, values and styles later. The
//! engine lays the tree out and handles input before your systems run; read
//! what happened with `crabby_ctx_ui_next_event`, one event at a time, so no
//! callback borrows your game state. Nodes are named by `uint64_t` ids that are
//! never reused: ids of removed nodes fail with `CRABBY_NOT_FOUND`.
//!
//! Layout is a flat `CrabbyUiLayout` (a Taffy style: flex, grid and absolute
//! positioning); start from `crabby_ui_layout_default()`. Looks are a
//! `CrabbyUiPaint`. Enum-valued fields are `uint32_t` holding the constants of
//! the matching `CrabbyUi*` enum, so a bad value is an error, not undefined.
//!
//! Tree operations take a `target`: `CRABBY_UI_SCREEN` for the screen's own UI, or an
//! entity with a world-space panel (`crabby_ctx_create_ui_panel`) to work on
//! that panel's UI with the same calls. Node ids belong to the UI that made them.
//! Text insertion and keyboard capture are context-wide input operations.
use std::ffi::c_char;

use crabbyengine::ui_layout::{
    AlignContent, AlignItems, Dimension, Display, FlexDirection, FlexWrap, GridPlacement, GridTemplateComponent,
    LengthPercentage, LengthPercentageAuto, Line, Position, Rect as Edges, Size, Style, TrackSizingFunction,
    style_helpers::{TaffyGridLine, TaffyGridSpan},
};
use crabbyengine::{Color, Entity, Ui, UiEvent, UiId, UiPaint, UiPanel3D, UiTextAlign, UiWidget};

use crate::ctx::{ctx_access, out, text};
use crate::media::{font_from_c, font_to_c};
use crate::types::{CrabbyColor, CrabbyRect, CrabbyTexture, CrabbyVec2, texture_from_c, texture_to_c, to_c};
use crate::{CrabbyCtx, CrabbyFont, CrabbyStatus, Failure, boundary, error, invalid};

/// The id that means "no node" (a missing parent, an empty screen stack, no
/// focus).
/// ```
/// assert_eq!(crabby_ffi::CRABBY_UI_NONE, u64::MAX);
/// ```
pub const CRABBY_UI_NONE: u64 = u64::MAX;

/// The `target` of the UI calls that means the screen's own UI. Any other
/// target is an entity with a world-space panel (`crabby_ctx_create_ui_panel`),
/// whose UI the call works on instead. Node ids are local to a UI: always use
/// the target that created them. The same numeric id can exist in several UIs.
/// ```
/// assert_eq!(crabby_ffi::CRABBY_UI_SCREEN, u64::MAX);
/// ```
pub const CRABBY_UI_SCREEN: u64 = u64::MAX;

/// Which layout algorithm a node uses (`CrabbyUiLayout::display`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiDisplay::CRABBY_UI_DISPLAY_GRID as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiDisplay {
    /// Flexbox (the default).
    CRABBY_UI_DISPLAY_FLEX,
    /// Grid; set the tracks with `grid_columns` and `grid_rows`.
    CRABBY_UI_DISPLAY_GRID,
    /// Hidden and out of layout.
    CRABBY_UI_DISPLAY_NONE,
}
const DISPLAYS: [Display; 3] = [Display::Flex, Display::Grid, Display::None];

/// How a node is positioned (`CrabbyUiLayout::position`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE as u32, 2);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiPosition {
    /// In normal flow; `inset` has no effect (the default).
    CRABBY_UI_POSITION_STATIC,
    /// In normal flow, then nudged by `inset` without moving its siblings. A
    /// relative node is also what its absolute children are placed inside.
    CRABBY_UI_POSITION_RELATIVE,
    /// Out of flow, placed by `inset` inside its closest relative, absolute or
    /// fixed ancestor (or the screen).
    CRABBY_UI_POSITION_ABSOLUTE,
    /// Out of flow, placed by `inset` relative to the screen.
    CRABBY_UI_POSITION_FIXED,
}
const POSITIONS: [Position; 4] = [
    Position::Static,
    Position::Relative,
    Position::Absolute,
    Position::Fixed,
];

/// Main axis of a flex container (`CrabbyUiLayout::flex_direction`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiFlexDirection {
    /// Left to right (the default).
    CRABBY_UI_FLEX_ROW,
    /// Top to bottom.
    CRABBY_UI_FLEX_COLUMN,
    /// Right to left.
    CRABBY_UI_FLEX_ROW_REVERSE,
    /// Bottom to top.
    CRABBY_UI_FLEX_COLUMN_REVERSE,
}
const DIRECTIONS: [FlexDirection; 4] = [
    FlexDirection::Row,
    FlexDirection::Column,
    FlexDirection::RowReverse,
    FlexDirection::ColumnReverse,
];

/// Whether flex items wrap onto more lines (`CrabbyUiLayout::flex_wrap`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiFlexWrap {
    /// One line (the default).
    CRABBY_UI_FLEX_NO_WRAP,
    /// Wrap onto more lines.
    CRABBY_UI_FLEX_WRAP,
    /// Wrap in the opposite direction.
    CRABBY_UI_FLEX_WRAP_REVERSE,
}
const WRAPS: [FlexWrap; 3] = [FlexWrap::NoWrap, FlexWrap::Wrap, FlexWrap::WrapReverse];

/// Alignment of items on one axis (`align_items`, `align_self`,
/// `justify_items`, `justify_self`). For the `_self` fields, `NORMAL` means
/// "use the parent's setting".
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiAlign::CRABBY_UI_ALIGN_CENTER as u32, 5);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiAlign {
    /// The layout mode's default (usually stretch).
    CRABBY_UI_ALIGN_NORMAL,
    /// Start of the axis.
    CRABBY_UI_ALIGN_START,
    /// End of the axis.
    CRABBY_UI_ALIGN_END,
    /// Start in flex direction.
    CRABBY_UI_ALIGN_FLEX_START,
    /// End in flex direction.
    CRABBY_UI_ALIGN_FLEX_END,
    /// Centered.
    CRABBY_UI_ALIGN_CENTER,
    /// Aligned text baselines.
    CRABBY_UI_ALIGN_BASELINE,
    /// Fill the axis.
    CRABBY_UI_ALIGN_STRETCH,
}
const ALIGNS: [AlignItems; 8] = [
    AlignItems::NORMAL,
    AlignItems::START,
    AlignItems::END,
    AlignItems::FLEX_START,
    AlignItems::FLEX_END,
    AlignItems::CENTER,
    AlignItems::BASELINE,
    AlignItems::STRETCH,
];

/// Distribution of lines or items (`align_content`, `justify_content`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN as u32, 7);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiAlignContent {
    /// The layout mode's default.
    CRABBY_UI_ALIGN_CONTENT_NORMAL,
    /// Packed at the start.
    CRABBY_UI_ALIGN_CONTENT_START,
    /// Packed at the end.
    CRABBY_UI_ALIGN_CONTENT_END,
    /// Packed at the start in flex direction.
    CRABBY_UI_ALIGN_CONTENT_FLEX_START,
    /// Packed at the end in flex direction.
    CRABBY_UI_ALIGN_CONTENT_FLEX_END,
    /// Centered.
    CRABBY_UI_ALIGN_CONTENT_CENTER,
    /// Stretched to fill.
    CRABBY_UI_ALIGN_CONTENT_STRETCH,
    /// Flush edges, even gaps.
    CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN,
    /// Even gaps including the edges.
    CRABBY_UI_ALIGN_CONTENT_SPACE_EVENLY,
    /// Half gaps at the edges.
    CRABBY_UI_ALIGN_CONTENT_SPACE_AROUND,
}
const CONTENT_ALIGNS: [AlignContent; 10] = [
    AlignContent::NORMAL,
    AlignContent::START,
    AlignContent::END,
    AlignContent::FLEX_START,
    AlignContent::FLEX_END,
    AlignContent::CENTER,
    AlignContent::STRETCH,
    AlignContent::SPACE_BETWEEN,
    AlignContent::SPACE_EVENLY,
    AlignContent::SPACE_AROUND,
];

/// How a `CrabbyUiLength` is read (`CrabbyUiLength::kind`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT as u32, 2);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiLengthKind {
    /// Chosen by the layout algorithm. For padding and gap this means 0.
    CRABBY_UI_LENGTH_AUTO,
    /// Logical units.
    CRABBY_UI_LENGTH_POINTS,
    /// A fraction of the parent (0.5 is half).
    CRABBY_UI_LENGTH_PERCENT,
}

/// How a `CrabbyUiTrack` is read (`CrabbyUiTrack::kind`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiTrackKind::CRABBY_UI_TRACK_FR as u32, 3);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiTrackKind {
    /// Sized by its content and the free space.
    CRABBY_UI_TRACK_AUTO,
    /// Logical units.
    CRABBY_UI_TRACK_POINTS,
    /// A fraction of the grid (0.5 is half).
    CRABBY_UI_TRACK_PERCENT,
    /// A share of the free space, like CSS `fr`.
    CRABBY_UI_TRACK_FR,
    /// As narrow as the content allows.
    CRABBY_UI_TRACK_MIN_CONTENT,
    /// As wide as the content wants.
    CRABBY_UI_TRACK_MAX_CONTENT,
}

/// Where a grid item starts or ends (`CrabbyUiGridLine::kind`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN as u32, 2);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiGridLineKind {
    /// Placed by the grid's auto-placement.
    CRABBY_UI_GRID_AUTO,
    /// At a grid line, counted from 1 (negative counts from the end).
    CRABBY_UI_GRID_LINE,
    /// Across this many tracks (1 or more).
    CRABBY_UI_GRID_SPAN,
}

/// How text lines line up inside a node (`CrabbyUiPaint::align`).
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_CENTER as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiTextAlign {
    /// Start of each line (the default).
    CRABBY_UI_TEXT_ALIGN_START,
    /// Center each line.
    CRABBY_UI_TEXT_ALIGN_CENTER,
    /// End of each line.
    CRABBY_UI_TEXT_ALIGN_END,
}
const TEXT_ALIGNS: [UiTextAlign; 3] = [UiTextAlign::Start, UiTextAlign::Center, UiTextAlign::End];

/// Which kind of event a `CrabbyUiEvent` is.
/// ```
/// assert_eq!(crabby_ffi::CrabbyUiEventKind::CRABBY_UI_EVENT_SCREEN_POPPED as u32, 3);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiEventKind {
    /// A button was activated (`id` is the button).
    #[default]
    CRABBY_UI_EVENT_CLICKED,
    /// A text input, toggle or slider changed (`id` is the node; read its value).
    CRABBY_UI_EVENT_CHANGED,
    /// Enter was pressed in a text input (`id` is the input).
    CRABBY_UI_EVENT_SUBMITTED,
    /// Cancel popped a screen (`id` is the screen's root).
    CRABBY_UI_EVENT_SCREEN_POPPED,
}

/// A layout length. `kind` is a `CrabbyUiLengthKind`.
/// ```
/// let l = crabby_ffi::CrabbyUiLength { kind: crabby_ffi::CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS as u32, value: 12.0 };
/// assert_eq!(l.value, 12.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiLength {
    /// A `CrabbyUiLengthKind`.
    pub kind: u32,
    /// Units, or a fraction (0.5 = 50%), depending on `kind`.
    pub value: f32,
}

/// One length per side of a box.
/// ```
/// let s = crabby_ffi::crabby_ui_layout_default().padding;
/// assert_eq!(s.left.value, 0.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiSides {
    /// Top side.
    pub top: CrabbyUiLength,
    /// Right side.
    pub right: CrabbyUiLength,
    /// Bottom side.
    pub bottom: CrabbyUiLength,
    /// Left side.
    pub left: CrabbyUiLength,
}

/// Space between rows and columns of children.
/// ```
/// let g = crabby_ffi::crabby_ui_layout_default().gap;
/// assert_eq!(g.row.value, 0.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiGap {
    /// Between columns.
    pub column: CrabbyUiLength,
    /// Between rows.
    pub row: CrabbyUiLength,
}

/// One grid row or column. `kind` is a `CrabbyUiTrackKind`.
/// ```
/// let t = crabby_ffi::CrabbyUiTrack { kind: crabby_ffi::CrabbyUiTrackKind::CRABBY_UI_TRACK_FR as u32, value: 1.0 };
/// assert_eq!(t.value, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiTrack {
    /// A `CrabbyUiTrackKind`.
    pub kind: u32,
    /// Units, a fraction, or an `fr` share, depending on `kind`.
    pub value: f32,
}

/// Where a grid item starts or ends on one axis. `kind` is a
/// `CrabbyUiGridLineKind`.
/// ```
/// let l = crabby_ffi::crabby_ui_layout_default().grid_column_start;
/// assert_eq!(l.kind, crabby_ffi::CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO as u32);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiGridLine {
    /// A `CrabbyUiGridLineKind`.
    pub kind: u32,
    /// The line number, or the span; ignored for auto.
    pub value: i32,
}

/// A node's layout: the Rust API's Taffy `Style`, flattened. Start from
/// `crabby_ui_layout_default()` and change what you need. Enum-valued fields
/// are `uint32_t` holding a `CrabbyUi*` constant. The track arrays are only
/// read during the call that takes the layout; a null array with a count of 0
/// means "none".
/// ```
/// let l = crabby_ffi::crabby_ui_layout_default();
/// assert_eq!(l.flex_shrink, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyUiLayout {
    /// A `CrabbyUiDisplay`. Default flex.
    pub display: u32,
    /// A `CrabbyUiPosition`. Default static.
    pub position: u32,
    /// A `CrabbyUiFlexDirection`. Default row.
    pub flex_direction: u32,
    /// A `CrabbyUiFlexWrap`. Default no wrap.
    pub flex_wrap: u32,
    /// Share of the free main-axis space this item takes. Default 0.
    pub flex_grow: f32,
    /// How much this item shrinks when space is short. Default 1.
    pub flex_shrink: f32,
    /// Main-axis size before growing or shrinking. Default auto.
    pub flex_basis: CrabbyUiLength,
    /// A `CrabbyUiAlign`: cross-axis alignment of children.
    pub align_items: u32,
    /// A `CrabbyUiAlign`: this item's own cross-axis alignment; normal inherits.
    pub align_self: u32,
    /// A `CrabbyUiAlign`: grid items' alignment along the row axis.
    pub justify_items: u32,
    /// A `CrabbyUiAlign`: this item's own row-axis alignment; normal inherits.
    pub justify_self: u32,
    /// A `CrabbyUiAlignContent`: how lines or tracks share the cross axis.
    pub align_content: u32,
    /// A `CrabbyUiAlignContent`: how items share the main axis.
    pub justify_content: u32,
    /// Width. Default auto.
    pub width: CrabbyUiLength,
    /// Height. Default auto.
    pub height: CrabbyUiLength,
    /// Smallest width. Default auto.
    pub min_width: CrabbyUiLength,
    /// Smallest height. Default auto.
    pub min_height: CrabbyUiLength,
    /// Largest width. Default auto (none).
    pub max_width: CrabbyUiLength,
    /// Largest height. Default auto (none).
    pub max_height: CrabbyUiLength,
    /// Outside space; auto margins absorb free space. Default 0.
    pub margin: CrabbyUiSides,
    /// Inside space (auto means 0). Default 0.
    pub padding: CrabbyUiSides,
    /// Offsets for relative and absolute positioning. Default auto.
    pub inset: CrabbyUiSides,
    /// Space between children (auto means 0). Default 0.
    pub gap: CrabbyUiGap,
    /// Width divided by height, or 0 for none. Default 0.
    pub aspect_ratio: f32,
    /// Explicit grid columns (`display` grid).
    pub grid_columns: *const CrabbyUiTrack,
    /// Number of `grid_columns`.
    pub grid_column_count: usize,
    /// Explicit grid rows.
    pub grid_rows: *const CrabbyUiTrack,
    /// Number of `grid_rows`.
    pub grid_row_count: usize,
    /// Size of columns added beyond `grid_columns`; they repeat. Default auto.
    pub grid_auto_columns: *const CrabbyUiTrack,
    /// Number of `grid_auto_columns`.
    pub grid_auto_column_count: usize,
    /// Size of rows added beyond `grid_rows`; they repeat. Default auto.
    pub grid_auto_rows: *const CrabbyUiTrack,
    /// Number of `grid_auto_rows`.
    pub grid_auto_row_count: usize,
    /// A grid item's first column.
    pub grid_column_start: CrabbyUiGridLine,
    /// A grid item's last column line.
    pub grid_column_end: CrabbyUiGridLine,
    /// A grid item's first row.
    pub grid_row_start: CrabbyUiGridLine,
    /// A grid item's last row line.
    pub grid_row_end: CrabbyUiGridLine,
}

/// A node's looks, independent of layout (Rust `UiPaint`).
/// ```
/// let p = crabby_ffi::crabby_ui_paint_default();
/// assert_eq!(p.font_size, 18.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiPaint {
    /// Normal background.
    pub background: CrabbyColor,
    /// Background while hovered or focused.
    pub hover: CrabbyColor,
    /// Background while the pointer is held on it.
    pub pressed: CrabbyColor,
    /// Background while disabled.
    pub disabled: CrabbyColor,
    /// Border and focus-ring color.
    pub border: CrabbyColor,
    /// Border thickness in logical units.
    pub border_width: f32,
    /// Text and image tint.
    pub color: CrabbyColor,
    /// Text font, from `crabby_ctx_load_font`; `{0}` is the built-in font.
    pub font: CrabbyFont,
    /// Text line height in logical units.
    pub font_size: f32,
    /// Break text at word boundaries.
    pub wrap: bool,
    /// A `CrabbyUiTextAlign`.
    pub align: u32,
    /// Draw `texture` as the image or background.
    pub has_texture: bool,
    /// Image or background; ignored unless `has_texture`.
    pub texture: CrabbyTexture,
    /// Source border in pixels for nine-slice drawing; 0 stretches the image.
    pub slice: f32,
}

/// One thing that happened, from `crabby_ctx_ui_next_event`.
/// ```
/// let e = crabby_ffi::CrabbyUiEvent { kind: crabby_ffi::CrabbyUiEventKind::CRABBY_UI_EVENT_CLICKED, id: 3 };
/// assert_eq!(e.id, 3);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrabbyUiEvent {
    /// What happened.
    pub kind: CrabbyUiEventKind,
    /// The node it happened to (the root, for a popped screen).
    pub id: u64,
}

/// One frame of input in logical window coordinates, for `crabby_ctx_ui_frame`.
/// Start from `crabby_ui_input_default()`.
/// ```
/// let i = crabby_ffi::crabby_ui_input_default();
/// assert!(!i.pressed && i.text.is_null());
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiInput {
    /// Pointer position.
    pub pointer: CrabbyVec2,
    /// The primary button went down this frame.
    pub pressed: bool,
    /// The primary button is down.
    pub held: bool,
    /// The primary button went up this frame.
    pub released: bool,
    /// Wheel movement (positive scrolls down and right).
    pub wheel: CrabbyVec2,
    /// Tab: focus the next control.
    pub next: bool,
    /// Shift-tab: focus the previous control.
    pub previous: bool,
    /// Arrows or d-pad (-1..1 per axis): move focus, the caret or a slider.
    pub direction: CrabbyVec2,
    /// Enter, space or gamepad south.
    pub confirm: bool,
    /// Escape or gamepad east: pops the top screen.
    pub cancel: bool,
    /// Shift is held: extend the text selection.
    pub shift: bool,
    /// Select all text in the focused input.
    pub select_all: bool,
    /// Delete before the caret.
    pub backspace: bool,
    /// Delete after the caret.
    pub delete: bool,
    /// Move the caret to the start.
    pub home: bool,
    /// Move the caret to the end.
    pub end: bool,
    /// Typed text, NUL-terminated and read only during the call; null for none.
    pub text: *const c_char,
}

// ---- defaults ----------------------------------------------------------------

const fn length(kind: CrabbyUiLengthKind, value: f32) -> CrabbyUiLength {
    CrabbyUiLength {
        kind: kind as u32,
        value,
    }
}
const AUTO: CrabbyUiLength = length(CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO, 0.0);
const ZERO: CrabbyUiLength = length(CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS, 0.0);
const fn sides(all: CrabbyUiLength) -> CrabbyUiSides {
    CrabbyUiSides {
        top: all,
        right: all,
        bottom: all,
        left: all,
    }
}
const NO_LINE: CrabbyUiGridLine = CrabbyUiGridLine {
    kind: CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO as u32,
    value: 0,
};

/// The default layout: a flex row, static, automatic size, no spacing (same
/// as Rust's `Style::default()`).
/// ```
/// let l = crabby_ffi::crabby_ui_layout_default();
/// assert_eq!(l.display, crabby_ffi::CrabbyUiDisplay::CRABBY_UI_DISPLAY_FLEX as u32);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_default() -> CrabbyUiLayout {
    CrabbyUiLayout {
        display: CrabbyUiDisplay::CRABBY_UI_DISPLAY_FLEX as u32,
        position: CrabbyUiPosition::CRABBY_UI_POSITION_STATIC as u32,
        flex_direction: CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW as u32,
        flex_wrap: CrabbyUiFlexWrap::CRABBY_UI_FLEX_NO_WRAP as u32,
        flex_grow: 0.0,
        flex_shrink: 1.0,
        flex_basis: AUTO,
        align_items: CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL as u32,
        align_self: CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL as u32,
        justify_items: CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL as u32,
        justify_self: CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL as u32,
        align_content: CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_NORMAL as u32,
        justify_content: CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_NORMAL as u32,
        width: AUTO,
        height: AUTO,
        min_width: AUTO,
        min_height: AUTO,
        max_width: AUTO,
        max_height: AUTO,
        margin: sides(ZERO),
        padding: sides(ZERO),
        inset: sides(AUTO),
        gap: CrabbyUiGap {
            column: ZERO,
            row: ZERO,
        },
        aspect_ratio: 0.0,
        grid_columns: std::ptr::null(),
        grid_column_count: 0,
        grid_rows: std::ptr::null(),
        grid_row_count: 0,
        grid_auto_columns: std::ptr::null(),
        grid_auto_column_count: 0,
        grid_auto_rows: std::ptr::null(),
        grid_auto_row_count: 0,
        grid_column_start: NO_LINE,
        grid_column_end: NO_LINE,
        grid_row_start: NO_LINE,
        grid_row_end: NO_LINE,
    }
}

/// The default paint (same as Rust's `UiPaint::default()`): the engine's UI
/// colors, the built-in font at 18 units, no border or image.
/// ```
/// let p = crabby_ffi::crabby_ui_paint_default();
/// assert_eq!(p.align, crabby_ffi::CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_START as u32);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_paint_default() -> CrabbyUiPaint {
    paint_to_c(&UiPaint::default())
}

/// Input with nothing pressed: no pointer movement, keys or text.
/// ```
/// assert_eq!(crabby_ffi::crabby_ui_input_default().pointer.x, 0.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_input_default() -> CrabbyUiInput {
    CrabbyUiInput {
        pointer: CrabbyVec2::default(),
        pressed: false,
        held: false,
        released: false,
        wheel: CrabbyVec2::default(),
        next: false,
        previous: false,
        direction: CrabbyVec2::default(),
        confirm: false,
        cancel: false,
        shift: false,
        select_all: false,
        backspace: false,
        delete: false,
        home: false,
        end: false,
        text: std::ptr::null(),
    }
}

// ---- conversions -------------------------------------------------------------

fn pick<T: Copy>(table: &[T], value: u32, message: &'static str) -> Result<T, Failure> {
    table.get(value as usize).copied().ok_or_else(|| invalid(message))
}

fn num(value: f32) -> Result<f32, Failure> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid("UI numbers must be finite"))
    }
}

fn dimension(l: CrabbyUiLength) -> Result<Dimension, Failure> {
    Ok(match pick(&KINDS, l.kind, "unknown UI length kind")? {
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO => Dimension::auto(),
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS => Dimension::length(num(l.value)?),
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT => Dimension::percent(num(l.value)?),
    })
}
const KINDS: [CrabbyUiLengthKind; 3] = [
    CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO,
    CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS,
    CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT,
];

fn length_percentage(l: CrabbyUiLength) -> Result<LengthPercentage, Failure> {
    Ok(match pick(&KINDS, l.kind, "unknown UI length kind")? {
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO => LengthPercentage::length(0.0),
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS => LengthPercentage::length(num(l.value)?),
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT => LengthPercentage::percent(num(l.value)?),
    })
}

fn length_percentage_auto(l: CrabbyUiLength) -> Result<LengthPercentageAuto, Failure> {
    Ok(match pick(&KINDS, l.kind, "unknown UI length kind")? {
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO => LengthPercentageAuto::auto(),
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS => LengthPercentageAuto::length(num(l.value)?),
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT => LengthPercentageAuto::percent(num(l.value)?),
    })
}

fn edges<T>(s: CrabbyUiSides, f: impl Fn(CrabbyUiLength) -> Result<T, Failure>) -> Result<Edges<T>, Failure> {
    Ok(Edges {
        left: f(s.left)?,
        right: f(s.right)?,
        top: f(s.top)?,
        bottom: f(s.bottom)?,
    })
}

fn track(t: CrabbyUiTrack) -> Result<TrackSizingFunction, Failure> {
    use crabbyengine::ui_layout::style_helpers as h;
    Ok(match pick(&TRACKS, t.kind, "unknown UI track kind")? {
        CrabbyUiTrackKind::CRABBY_UI_TRACK_AUTO => h::auto(),
        CrabbyUiTrackKind::CRABBY_UI_TRACK_POINTS => h::length(num(t.value)?),
        CrabbyUiTrackKind::CRABBY_UI_TRACK_PERCENT => h::percent(num(t.value)?),
        CrabbyUiTrackKind::CRABBY_UI_TRACK_FR => h::fr(num(t.value)?),
        CrabbyUiTrackKind::CRABBY_UI_TRACK_MIN_CONTENT => h::min_content(),
        CrabbyUiTrackKind::CRABBY_UI_TRACK_MAX_CONTENT => h::max_content(),
    })
}
const TRACKS: [CrabbyUiTrackKind; 6] = [
    CrabbyUiTrackKind::CRABBY_UI_TRACK_AUTO,
    CrabbyUiTrackKind::CRABBY_UI_TRACK_POINTS,
    CrabbyUiTrackKind::CRABBY_UI_TRACK_PERCENT,
    CrabbyUiTrackKind::CRABBY_UI_TRACK_FR,
    CrabbyUiTrackKind::CRABBY_UI_TRACK_MIN_CONTENT,
    CrabbyUiTrackKind::CRABBY_UI_TRACK_MAX_CONTENT,
];

/// Reads `count` tracks at `ptr` (null with count 0 is none).
///
/// # Safety
/// `ptr` is null or points to `count` tracks.
unsafe fn track_list(ptr: *const CrabbyUiTrack, count: usize) -> Result<Vec<TrackSizingFunction>, Failure> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() {
        return Err(invalid("UI grid tracks are null"));
    }
    // SAFETY: caller contract: `count` readable tracks.
    unsafe { std::slice::from_raw_parts(ptr, count) }
        .iter()
        .map(|&t| track(t))
        .collect()
}

fn placement(l: CrabbyUiGridLine) -> Result<GridPlacement, Failure> {
    const KINDS: [CrabbyUiGridLineKind; 3] = [
        CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO,
        CrabbyUiGridLineKind::CRABBY_UI_GRID_LINE,
        CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN,
    ];
    Ok(match pick(&KINDS, l.kind, "unknown UI grid line kind")? {
        CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO => GridPlacement::Auto,
        CrabbyUiGridLineKind::CRABBY_UI_GRID_LINE => match i16::try_from(l.value) {
            Ok(line) if line != 0 => GridPlacement::from_line_index(line),
            _ => return Err(invalid("UI grid line must be a nonzero 16-bit number")),
        },
        CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN => match u16::try_from(l.value) {
            Ok(span) if span != 0 => GridPlacement::from_span(span),
            _ => return Err(invalid("UI grid span must be from 1 to 65535")),
        },
    })
}

/// Checks a C layout and converts it, so bad values are errors, not panics.
///
/// # Safety
/// The track pointers are null or point to their counts of tracks.
unsafe fn layout_style(l: &CrabbyUiLayout) -> Result<Style, Failure> {
    // SAFETY: caller contract.
    let (columns, rows, auto_columns, auto_rows) = unsafe {
        (
            track_list(l.grid_columns, l.grid_column_count)?,
            track_list(l.grid_rows, l.grid_row_count)?,
            track_list(l.grid_auto_columns, l.grid_auto_column_count)?,
            track_list(l.grid_auto_rows, l.grid_auto_row_count)?,
        )
    };
    let aspect_ratio = num(l.aspect_ratio)?;
    if aspect_ratio < 0.0 {
        return Err(invalid("UI aspect ratio must not be negative"));
    }
    let own = |value| -> Result<Option<AlignItems>, Failure> {
        (value != CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL as u32)
            .then(|| pick(&ALIGNS, value, "unknown UI alignment"))
            .transpose()
    };
    Ok(Style {
        display: pick(&DISPLAYS, l.display, "unknown UI display")?,
        position: pick(&POSITIONS, l.position, "unknown UI position")?,
        flex_direction: pick(&DIRECTIONS, l.flex_direction, "unknown UI flex direction")?,
        flex_wrap: pick(&WRAPS, l.flex_wrap, "unknown UI flex wrap")?,
        flex_grow: num(l.flex_grow)?,
        flex_shrink: num(l.flex_shrink)?,
        flex_basis: dimension(l.flex_basis)?,
        align_items: pick(&ALIGNS, l.align_items, "unknown UI alignment")?,
        align_self: own(l.align_self)?,
        justify_items: pick(&ALIGNS, l.justify_items, "unknown UI alignment")?,
        justify_self: own(l.justify_self)?,
        align_content: pick(&CONTENT_ALIGNS, l.align_content, "unknown UI content alignment")?,
        justify_content: pick(&CONTENT_ALIGNS, l.justify_content, "unknown UI content alignment")?,
        size: Size {
            width: dimension(l.width)?,
            height: dimension(l.height)?,
        },
        min_size: Size {
            width: length_percentage_auto(l.min_width)?,
            height: length_percentage_auto(l.min_height)?,
        },
        max_size: Size {
            width: length_percentage_auto(l.max_width)?,
            height: length_percentage_auto(l.max_height)?,
        },
        margin: edges(l.margin, length_percentage_auto)?,
        padding: edges(l.padding, length_percentage)?,
        inset: edges(l.inset, length_percentage_auto)?,
        gap: Size {
            width: length_percentage(l.gap.column)?,
            height: length_percentage(l.gap.row)?,
        },
        aspect_ratio: (aspect_ratio > 0.0).then_some(aspect_ratio),
        grid_template_columns: columns.into_iter().map(GridTemplateComponent::Single).collect(),
        grid_template_rows: rows.into_iter().map(GridTemplateComponent::Single).collect(),
        grid_auto_columns: auto_columns.into_iter().collect(),
        grid_auto_rows: auto_rows.into_iter().collect(),
        grid_column: Line {
            start: placement(l.grid_column_start)?,
            end: placement(l.grid_column_end)?,
        },
        grid_row: Line {
            start: placement(l.grid_row_start)?,
            end: placement(l.grid_row_end)?,
        },
        ..Style::default()
    })
}

/// The style for an optional C layout (null is the default).
///
/// # Safety
/// `layout` is null or a readable layout whose track pointers are valid.
unsafe fn style_or_default(layout: *const CrabbyUiLayout) -> Result<Style, Failure> {
    // SAFETY: caller contract.
    match unsafe { layout.as_ref() } {
        Some(layout) => unsafe { layout_style(layout) },
        None => Ok(Style::default()),
    }
}

fn paint_to_c(p: &UiPaint) -> CrabbyUiPaint {
    let (has_texture, texture) = match p.texture {
        Some(t) => (true, texture_to_c(t)),
        None => (
            false,
            CrabbyTexture {
                id: 0,
                width: 0,
                height: 0,
            },
        ),
    };
    CrabbyUiPaint {
        background: to_c(p.background),
        hover: to_c(p.hover),
        pressed: to_c(p.pressed),
        disabled: to_c(p.disabled),
        border: to_c(p.border),
        border_width: p.border_width,
        color: to_c(p.color),
        font: font_to_c(p.font),
        font_size: p.font_size,
        wrap: p.wrap,
        align: TEXT_ALIGNS.iter().position(|&a| a == p.align).unwrap_or(0) as u32,
        has_texture,
        texture,
        slice: p.slice,
    }
}

fn paint_from_c(p: &CrabbyUiPaint) -> Result<UiPaint, Failure> {
    let (border_width, font_size, slice) = (num(p.border_width)?, num(p.font_size)?, num(p.slice)?);
    if border_width < 0.0 || font_size < 0.0 || slice < 0.0 {
        return Err(invalid("UI border width, font size and slice must not be negative"));
    }
    Ok(UiPaint {
        background: to_c(p.background),
        hover: to_c(p.hover),
        pressed: to_c(p.pressed),
        disabled: to_c(p.disabled),
        border: to_c(p.border),
        border_width,
        color: to_c(p.color),
        font: font_from_c(p.font),
        font_size,
        wrap: p.wrap,
        align: pick(&TEXT_ALIGNS, p.align, "unknown UI text alignment")?,
        texture: p.has_texture.then(|| texture_from_c(p.texture)),
        slice,
    })
}

/// A string argument where null means empty.
///
/// # Safety
/// `ptr` is null or NUL-terminated.
unsafe fn optional_text<'a>(ptr: *const c_char) -> Result<&'a str, Failure> {
    if ptr.is_null() {
        Ok("")
    } else {
        // SAFETY: caller contract.
        unsafe { text(ptr) }
    }
}

fn existing(ui: &Ui, id: u64) -> Result<UiId, Failure> {
    let id = UiId::from_bits(id);
    if ui.rect(id).is_some() {
        Ok(id)
    } else {
        Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "UI node not found"))
    }
}

/// Fails for a font the engine doesn't know, which would otherwise panic
/// later, when the UI is laid out.
fn check_font(c: &mut crabbyengine::Ctx, font: crabbyengine::Font) -> Result<(), Failure> {
    if font == crabbyengine::Font::DEFAULT {
        return Ok(());
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.measure_text_ex("", font, 1.0)))
        .map(drop)
        .map_err(|_| invalid("unknown font"))
}

fn id_to_c(id: Option<UiId>) -> u64 {
    id.map_or(CRABBY_UI_NONE, UiId::bits)
}

/// The UI a `target` names: the screen's, or the one of the world-space panel
/// on an entity.
pub(crate) fn ui_of(c: &mut crabbyengine::Ctx, target: u64) -> Result<&mut Ui, Failure> {
    if target == CRABBY_UI_SCREEN {
        return Ok(&mut c.ui_retained);
    }
    let e = Entity::from_bits(target);
    if !c.world.is_alive(e) {
        return Err(invalid("no such entity"));
    }
    c.world
        .get_mut::<UiPanel3D>(e)
        .map(|panel| &mut panel.ui)
        .ok_or_else(|| error(CrabbyStatus::CRABBY_NOT_FOUND, "entity has no UI panel"))
}

/// Runs `f` on the UI of `target`.
///
/// # Safety
/// Forwards the caller's contract for `ctx`.
unsafe fn with_ui<R>(
    ctx: *mut CrabbyCtx,
    target: u64,
    f: impl FnOnce(&mut Ui) -> Result<R, Failure>,
) -> Result<R, Failure> {
    // SAFETY: forwarded caller contract.
    unsafe { ctx_access(ctx, |c| f(ui_of(c, target)?)) }
}

// ---- building the tree -------------------------------------------------------

/// What a new node starts with besides its layout.
struct Start<'a> {
    text: &'a str,
    paint: Option<UiPaint>,
    value: Option<f32>,
}

/// Adds a node under `parent` (or detached for `CRABBY_UI_NONE`). Everything
/// is checked before the tree changes, so a failed call adds nothing.
///
/// # Safety
/// Forwards the caller's contract for `ctx`, `layout` and `out_id`.
unsafe fn add(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    widget: UiWidget,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
    start: Start,
) -> Result<(), Failure> {
    if out_id.is_null() {
        return Err(invalid("output is null"));
    }
    // SAFETY: forwarded caller contract.
    let style = unsafe { style_or_default(layout)? };
    // SAFETY: as above.
    unsafe {
        with_ui(ctx, target, |ui| {
            let parent = (parent != CRABBY_UI_NONE).then(|| existing(ui, parent)).transpose()?;
            let id = match parent {
                Some(parent) => ui.add_child(parent, widget, style),
                None => ui.add(widget, style),
            };
            ui.set_text(id, start.text);
            if let Some(paint) = start.paint {
                ui.set_paint(id, paint);
            }
            if let Some(value) = start.value {
                ui.set_value(id, value);
            }
            out(out_id, id.bits())
        })
    }
}

fn plain(text: &str) -> Start<'_> {
    Start {
        text,
        paint: None,
        value: None,
    }
}

/// Adds a panel, a container for other nodes. `parent` is the node to add it
/// under, or `CRABBY_UI_NONE` for a detached root (to push as a screen). A null
/// `layout` is the default. Writes the new id to `out_id`.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_panel(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_panel(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { add(ctx, target, parent, UiWidget::Panel, layout, out_id, plain("")) })
}

/// Adds a label showing `text` (null for none). See `crabby_ctx_ui_add_panel`
/// for `parent`, `layout` and `out_id`.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_label(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, c"Hi".as_ptr(), std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` null or NUL-terminated; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_label(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        add(
            ctx,
            target,
            parent,
            UiWidget::Label,
            layout,
            out_id,
            plain(optional_text(text)?),
        )
    })
}

/// Adds a button labelled `text` (null for none). Activating it queues
/// `CRABBY_UI_EVENT_CLICKED`. See `crabby_ctx_ui_add_panel` for the rest.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_button(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, c"Buy".as_ptr(), std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` null or NUL-terminated; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_button(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        add(
            ctx,
            target,
            parent,
            UiWidget::Button,
            layout,
            out_id,
            plain(optional_text(text)?),
        )
    })
}

/// Adds an image showing `texture`, tinted by the paint's `color`. Change its
/// looks later with `crabby_ctx_ui_set_paint`. See `crabby_ctx_ui_add_panel`
/// for the rest.
/// ```
/// let t = crabby_ffi::CrabbyTexture { id: 0, width: 1, height: 1 };
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_image(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, t, std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_image(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    texture: CrabbyTexture,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // Like the Rust API, an image has no background of its own.
    let paint = UiPaint {
        background: Color::rgba(0.0, 0.0, 0.0, 0.0),
        texture: Some(texture_from_c(texture)),
        ..UiPaint::default()
    };
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        add(
            ctx,
            target,
            parent,
            UiWidget::Image,
            layout,
            out_id,
            Start {
                text: "",
                paint: Some(paint),
                value: None,
            },
        )
    })
}

/// Adds a scroll area: it clips its children and scrolls them with the wheel,
/// by dragging, and with a bar on the right. Give it a bounded size. See
/// `crabby_ctx_ui_add_panel` for the rest.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_scroll(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_scroll(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { add(ctx, target, parent, UiWidget::Scroll, layout, out_id, plain("")) })
}

/// Adds a single-line text input holding `text` (null for empty). Typing
/// queues `CRABBY_UI_EVENT_CHANGED`, Enter `CRABBY_UI_EVENT_SUBMITTED`. See
/// `crabby_ctx_ui_add_panel` for the rest.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_text_input(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` null or NUL-terminated; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_text_input(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        add(
            ctx,
            target,
            parent,
            UiWidget::TextInput,
            layout,
            out_id,
            plain(optional_text(text)?),
        )
    })
}

/// Adds a checkbox labelled `text` (null for none), starting `on`.
/// Toggling it queues `CRABBY_UI_EVENT_CHANGED`. See `crabby_ctx_ui_add_panel`
/// for the rest.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_toggle(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, std::ptr::null(), true, std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` null or NUL-terminated; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_toggle(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    on: bool,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let start = Start {
            text: optional_text(text)?,
            paint: None,
            value: Some(if on { 1.0 } else { 0.0 }),
        };
        add(ctx, target, parent, UiWidget::Toggle, layout, out_id, start)
    })
}

/// Adds a slider holding `value` from 0 to 1 (clamped). Dragging it, or the
/// left and right keys, queues `CRABBY_UI_EVENT_CHANGED`. See
/// `crabby_ctx_ui_add_panel` for the rest.
/// ```
/// let status = unsafe { crabby_ffi::crabby_ctx_ui_add_slider(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, crabby_ffi::CRABBY_UI_NONE, 0.5, std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_slider(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    value: f32,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let start = Start {
            text: "",
            paint: None,
            value: Some(value),
        };
        add(ctx, target, parent, UiWidget::Slider, layout, out_id, start)
    })
}

/// Removes a node and everything under it, and drops it from the screen stack.
/// Its ids stay invalid: later calls with them fail with `CRABBY_NOT_FOUND`.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_remove(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_remove(ctx: *mut CrabbyCtx, target: u64, id: u64) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.remove(id);
            Ok(())
        })
    })
}

// ---- changing nodes ----------------------------------------------------------

/// Replaces a node's layout. A null `layout` restores the default.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_layout(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `layout` null or readable, with valid track pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_layout(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    layout: *const CrabbyUiLayout,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let style = style_or_default(layout)?;
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.set_layout(id, style);
            Ok(())
        })
    })
}

/// Replaces a node's paint. A null `paint` restores the default. Fonts must
/// come from `crabby_ctx_load_font`; an unknown one is an error.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_paint(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `paint` null or readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_paint(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    paint: *const CrabbyUiPaint,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let paint = match paint.as_ref() {
            Some(paint) => paint_from_c(paint)?,
            None => UiPaint::default(),
        };
        ctx_access(ctx, |c| {
            check_font(c, paint.font)?;
            let ui = ui_of(c, target)?;
            let id = existing(ui, id)?;
            ui.set_paint(id, paint);
            Ok(())
        })
    })
}

/// Sets the text of a label, button, toggle or text input (null for empty).
/// Text inputs drop control characters. No event is queued.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_text(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, c"x".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` null or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_text(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    text: *const c_char,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let value = optional_text(text)?;
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.set_text(id, value);
            Ok(())
        })
    })
}

/// Reads a node's text (NUL-terminated) into `buffer` of `capacity` bytes and
/// writes its length, without the NUL, to `out_len`. With capacity <= length it
/// writes an empty string (if capacity > 0): allocate length + 1 and call
/// again. A null buffer is accepted only with capacity 0.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_text(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null_mut(), 0, &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `buffer` writable for `capacity` bytes (or null with
/// capacity 0); `out_len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_text(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        if out_len.is_null() || (buffer.is_null() && capacity != 0) {
            return Err(invalid("output is null"));
        }
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            let value = ui.text(id).unwrap_or("");
            // SAFETY: validated non-null outputs; the caller supplies the stated capacity.
            out_len.write(value.len());
            if value.len() < capacity {
                std::ptr::copy_nonoverlapping(value.as_ptr(), buffer.cast(), value.len());
                buffer.add(value.len()).write(0);
            } else if capacity != 0 {
                buffer.write(0);
            }
            Ok(())
        })
    })
}

/// Enables or disables a node and everything under it. Disabled controls take
/// no input and draw with the paint's `disabled` color.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_enabled(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, false) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_enabled(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    enabled: bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.set_enabled(id, enabled);
            Ok(())
        })
    })
}

/// Sets a slider's value (0 to 1, clamped) or a toggle's (0 off, 1 on). No
/// event is queued.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_value(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, 0.5) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_value(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.set_value(id, value);
            Ok(())
        })
    })
}

/// Reads a slider's or toggle's value; read it after a
/// `CRABBY_UI_EVENT_CHANGED` event.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_value(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, &mut 0.0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_value` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_value(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_value: *mut f32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            out(out_value, ui.value(id).unwrap_or(0.0))
        })
    })
}

/// Scrolls a scroll area to `offset` (logical units, unscaled); layout clamps
/// it to the content.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_scroll(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, crabby_ffi::CrabbyVec2 { x: 0.0, y: 40.0 }) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_scroll(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    offset: CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.set_scroll(id, to_c(offset));
            Ok(())
        })
    })
}

/// Reads a scroll area's offset as of the last layout.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_scroll(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, &mut Default::default()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_offset` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_scroll(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_offset: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            out(out_offset, to_c(ui.scroll(id).unwrap_or_default()))
        })
    })
}

// ---- screens and focus -------------------------------------------------------

/// Pushes a detached root as the top screen. Only the top screen takes input;
/// a `modal` one also dims the screens below it and owns the pointer outside
/// its rectangle. Fails with `CRABBY_INVALID_ARGUMENT` if the node has a parent
/// or is already a screen.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_push_screen(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, true) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_push_screen(
    ctx: *mut CrabbyCtx,
    target: u64,
    root: u64,
    modal: bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let root = existing(ui, root)?;
            if ui.parent(root).is_some() {
                return Err(invalid("UI screen must be a detached root"));
            }
            if ui.is_screen(root) {
                return Err(invalid("UI screen already on stack"));
            }
            ui.push_screen(root, modal);
            Ok(())
        })
    })
}

/// Pops the top screen and writes its root to `out_root` (`CRABBY_UI_NONE`
/// when the stack was empty). The nodes stay, so it can be pushed again. The
/// previous screen's focus comes back.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_pop_screen(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_root` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_pop_screen(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_root: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        if out_root.is_null() {
            return Err(invalid("output is null"));
        }
        with_ui(ctx, target, |ui| out(out_root, id_to_c(ui.pop_screen())))
    })
}

/// Writes the top screen's root to `out_root` (`CRABBY_UI_NONE` for none).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_top_screen(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_root` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_top_screen(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_root: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { with_ui(ctx, target, |ui| out(out_root, id_to_c(ui.top_screen()))) })
}

/// Writes the focused node (keyboard and gamepad) to `out_id`
/// (`CRABBY_UI_NONE` for none).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_focus(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_focus(ctx: *mut CrabbyCtx, target: u64, out_id: *mut u64) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { with_ui(ctx, target, |ui| out(out_id, id_to_c(ui.focus()))) })
}

/// Sets a user scale on top of the window's DPI scale (clamped to 0.25..8).
/// Geometry, text and hit testing all use it.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_set_scale(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 1.25) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_scale(ctx: *mut CrabbyCtx, target: u64, scale: f32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            ui.set_scale(scale);
            Ok(())
        })
    })
}

/// Reads a node's rectangle in logical window coordinates, as of the last layout.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_rect(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, 0, &mut Default::default()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_rect` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_rect(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_rect: *mut CrabbyRect,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            out(out_rect, to_c(ui.rect(id).unwrap_or_default()))
        })
    })
}

/// Writes the topmost enabled node at `point` on the top screen to `out_id`
/// (`CRABBY_UI_NONE` for none), as of the last layout.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_hit_test(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, Default::default(), &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_hit_test(
    ctx: *mut CrabbyCtx,
    target: u64,
    point: CrabbyVec2,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { with_ui(ctx, target, |ui| out(out_id, id_to_c(ui.hit_test(to_c(point))))) })
}

/// Whether the pointer belongs to the UI (over a control, dragging, or a modal
/// screen is open). Check it before acting on clicks in the world.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_wants_pointer(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, &mut false) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_wanted` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_wants_pointer(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_wanted: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { with_ui(ctx, target, |ui| out(out_wanted, ui.wants_pointer())) })
}

/// Whether the keyboard and gamepad belong to the UI (a screen is open).
/// Check it before acting on keys in the game.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_wants_keyboard(std::ptr::null_mut(), &mut false) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_wanted` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_wants_keyboard(ctx: *mut CrabbyCtx, out_wanted: *mut bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_wanted, c.ui_wants_keyboard())) })
}

// ---- events and frames -------------------------------------------------------

/// Takes the oldest queued event into `out_event`, or fails with
/// `CRABBY_NOT_FOUND` when none are left. Unread events expire at the next UI
/// frame, so drain them every frame. Removing a node drops its events.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_next_event(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, &mut crabby_ffi::CrabbyUiEvent { kind: Default::default(), id: 0 }) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_event` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_next_event(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_event: *mut CrabbyUiEvent,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        if out_event.is_null() {
            return Err(invalid("output is null"));
        }
        with_ui(ctx, target, |ui| {
            let Some(event) = ui.next_event() else {
                return Err(error(CrabbyStatus::CRABBY_NOT_FOUND, "no UI events left"));
            };
            let (kind, id) = match event {
                UiEvent::Clicked(id) => (CrabbyUiEventKind::CRABBY_UI_EVENT_CLICKED, id),
                UiEvent::Changed(id) => (CrabbyUiEventKind::CRABBY_UI_EVENT_CHANGED, id),
                UiEvent::Submitted(id) => (CrabbyUiEventKind::CRABBY_UI_EVENT_SUBMITTED, id),
                UiEvent::ScreenPopped(id) => (CrabbyUiEventKind::CRABBY_UI_EVENT_SCREEN_POPPED, id),
            };
            out(out_event, CrabbyUiEvent { kind, id: id.bits() })
        })
    })
}

/// Lays the tree out for a window of `viewport` logical units, without
/// processing input. The engine does this every frame; call it to read fresh
/// rectangles right after changing the tree.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_layout(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, Default::default()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_layout(ctx: *mut CrabbyCtx, target: u64, viewport: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            ui.layout(to_c(viewport));
            Ok(())
        })
    })
}

/// Lays the tree out and handles one frame of `input` (null for none): focus,
/// clicks, text editing, scrolling and the screen stack. The engine already does
/// this every frame from the real mouse and keyboard, and it starts a new frame
/// by dropping unread events. Call it yourself to drive the UI in headless
/// tests or from a custom host.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_frame(std::ptr::null_mut(), crabby_ffi::CRABBY_UI_SCREEN, Default::default(), std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `input` null or readable, its `text` null or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_frame(
    ctx: *mut CrabbyCtx,
    target: u64,
    viewport: CrabbyVec2,
    input: *const CrabbyUiInput,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let input = match input.as_ref() {
            Some(i) => crabbyengine::UiInput {
                pointer: to_c(i.pointer),
                pressed: i.pressed,
                held: i.held,
                released: i.released,
                wheel: to_c(i.wheel),
                next: i.next,
                previous: i.previous,
                direction: to_c(i.direction),
                confirm: i.confirm,
                cancel: i.cancel,
                shift: i.shift,
                select_all: i.select_all,
                backspace: i.backspace,
                delete: i.delete,
                home: i.home,
                end: i.end,
                text: optional_text(i.text)?.to_owned(),
            },
            None => crabbyengine::UiInput::default(),
        };
        with_ui(ctx, target, |ui| {
            ui.frame(to_c(viewport), &input);
            Ok(())
        })
    })
}

/// Feeds typed text to the next UI frame, for tests and custom hosts. Native
/// windows forward typed characters on their own.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_ui_insert_text(std::ptr::null_mut(), c"Hi".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_insert_text(ctx: *mut CrabbyCtx, text: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let value = crate::ctx::text(text)?;
        ctx_access(ctx, |c| {
            c.ui_insert_text(value);
            Ok(())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_is_the_rust_default() {
        // SAFETY: the default layout has null track pointers.
        let style = unsafe { layout_style(&crabby_ui_layout_default()) }.ok().unwrap();
        assert_eq!(style, Style::default());
    }

    #[test]
    fn default_paint_round_trips() {
        let c = crabby_ui_paint_default();
        assert_eq!(paint_to_c(&paint_from_c(&c).ok().unwrap()), c);
        assert_eq!(c.font_size, UiPaint::default().font_size);
    }

    #[test]
    fn bad_layout_values_are_errors() {
        let base = crabby_ui_layout_default();
        let bad = [
            CrabbyUiLayout { display: 9, ..base },
            CrabbyUiLayout { align_self: 99, ..base },
            CrabbyUiLayout {
                flex_grow: f32::NAN,
                ..base
            },
            CrabbyUiLayout {
                aspect_ratio: -1.0,
                ..base
            },
            CrabbyUiLayout {
                width: CrabbyUiLength { kind: 7, value: 1.0 },
                ..base
            },
            CrabbyUiLayout {
                grid_columns: std::ptr::null(),
                grid_column_count: 1,
                ..base
            },
            CrabbyUiLayout {
                grid_column_start: CrabbyUiGridLine { kind: 1, value: 0 },
                ..base
            },
            CrabbyUiLayout {
                grid_row_end: CrabbyUiGridLine { kind: 2, value: 0 },
                ..base
            },
        ];
        for layout in bad {
            // SAFETY: the bad track pointer has a count the check rejects first.
            assert!(unsafe { layout_style(&layout) }.is_err(), "{layout:?}");
        }
    }

    #[test]
    fn grid_tracks_and_lines_convert() {
        let tracks = [
            CrabbyUiTrack {
                kind: CrabbyUiTrackKind::CRABBY_UI_TRACK_FR as u32,
                value: 1.0,
            },
            CrabbyUiTrack {
                kind: CrabbyUiTrackKind::CRABBY_UI_TRACK_POINTS as u32,
                value: 50.0,
            },
        ];
        let layout = CrabbyUiLayout {
            display: CrabbyUiDisplay::CRABBY_UI_DISPLAY_GRID as u32,
            grid_columns: tracks.as_ptr(),
            grid_column_count: tracks.len(),
            grid_auto_rows: tracks[1..].as_ptr(),
            grid_auto_row_count: 1,
            grid_column_end: CrabbyUiGridLine {
                kind: CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN as u32,
                value: 2,
            },
            ..crabby_ui_layout_default()
        };
        // SAFETY: the track pointers cover their counts.
        let style = unsafe { layout_style(&layout) }.ok().unwrap();
        assert_eq!(style.display, Display::Grid);
        assert_eq!(style.grid_template_columns.len(), 2);
        assert_eq!(style.grid_auto_rows.len(), 1);
        assert_eq!(style.grid_column.end, GridPlacement::from_span(2));
    }

    #[test]
    fn ids_round_trip_through_integers() {
        assert_eq!(UiId::from_bits(UiId::from_bits(12).bits()).bits(), 12);
        assert_eq!(id_to_c(None), CRABBY_UI_NONE);
    }
}
