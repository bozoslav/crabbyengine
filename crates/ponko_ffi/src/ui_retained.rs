//! Retained UI from C (Rust `Ui`, `UiWidget`, `UiPaint`, `Ctx::ui`).
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

use ponko::ui::Ui;
use ponko::ui::UiEvent;
use ponko::ui::UiId;
use ponko::ui::UiPaint;
use ponko::ui::UiTextAlign;
use ponko::ui::UiTheme;
use ponko::ui::UiWidget;
use ponko::ui_layout::{
    AlignContent, AlignItems, Dimension, Display, FlexDirection, FlexWrap, GridPlacement, GridTemplateArea,
    GridTemplateAreas, GridTemplateComponent, GridTemplateRepetition, LengthPercentage, LengthPercentageAuto, Line,
    Position, Rect as Edges, RepetitionCount, Size, Style, TrackSizingFunction,
    style_helpers::{TaffyGridLine, TaffyGridSpan},
};
use ponko::{Color, Entity, d3::UiPanel as UiPanel3D};

use crate::ctx::{ctx_access, out, text};
use crate::media::{font_from_c, font_to_c};
use crate::types::{CrabbyColor, CrabbyRect, CrabbyTexture, CrabbyVec2, texture_from_c, texture_to_c, to_c};
use crate::{CrabbyCtx, CrabbyFont, CrabbyStatus, Failure, boundary, error, invalid};

/// The id that means "no node" (a missing parent, an empty screen stack, no
/// focus).
/// ```
/// assert_eq!(ponko_ffi::CRABBY_UI_NONE, u64::MAX);
/// ```
pub const CRABBY_UI_NONE: u64 = u64::MAX;

/// The `target` of the UI calls that means the screen's own UI. Any other
/// target is an entity with a world-space panel (`crabby_ctx_create_ui_panel`),
/// whose UI the call works on instead. Node ids are local to a UI: always use
/// the target that created them. The same numeric id can exist in several UIs.
/// ```
/// assert_eq!(ponko_ffi::CRABBY_UI_SCREEN, u64::MAX);
/// ```
pub const CRABBY_UI_SCREEN: u64 = u64::MAX;

/// Which layout algorithm a node uses (`CrabbyUiLayout::display`).
/// ```
/// assert_eq!(ponko_ffi::CrabbyUiDisplay::CRABBY_UI_DISPLAY_GRID as u32, 1);
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
/// assert_eq!(ponko_ffi::CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE as u32, 2);
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
/// assert_eq!(ponko_ffi::CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN as u32, 1);
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
/// assert_eq!(ponko_ffi::CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP as u32, 1);
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
/// assert_eq!(ponko_ffi::CrabbyUiAlign::CRABBY_UI_ALIGN_CENTER as u32, 5);
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
/// assert_eq!(ponko_ffi::CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN as u32, 7);
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
/// assert_eq!(ponko_ffi::CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT as u32, 2);
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
/// assert_eq!(ponko_ffi::CrabbyUiTrackKind::CRABBY_UI_TRACK_FR as u32, 3);
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
/// assert_eq!(ponko_ffi::CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN as u32, 2);
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
/// assert_eq!(ponko_ffi::CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_CENTER as u32, 1);
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
/// assert_eq!(ponko_ffi::CrabbyUiEventKind::CRABBY_UI_EVENT_SCREEN_POPPED as u32, 3);
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
/// let l = ponko_ffi::CrabbyUiLength { kind: ponko_ffi::CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS as u32, value: 12.0 };
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
/// let s = ponko_ffi::crabby_ui_layout_default().padding;
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
/// let g = ponko_ffi::crabby_ui_layout_default().gap;
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
/// let t = ponko_ffi::CrabbyUiTrack { kind: ponko_ffi::CrabbyUiTrackKind::CRABBY_UI_TRACK_FR as u32, value: 1.0 };
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

/// A template component: one track, or a non-nested repeat of tracks.
/// `repetition`: 0 = single (`track`), 1 = count, 2 = auto-fill, 3 = auto-fit.
/// ```
/// assert!(std::mem::size_of::<ponko_ffi::CrabbyUiTemplateTrack>() > 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyUiTemplateTrack {
    /// Single track; ignored for repeats.
    pub track: CrabbyUiTrack,
    /// 0 single, 1 counted repeat, 2 auto-fill, 3 auto-fit.
    pub repetition: u32,
    /// Positive repeat count (1..65535), used only for repetition 1.
    pub count: u32,
    /// Repeated tracks, copied during the call. No nested repeats.
    pub tracks: *const CrabbyUiTrack,
    /// Number of repeated tracks; must be positive for repeats.
    pub track_count: usize,
    /// Optional space-separated names per repeated line. Zero, or track_count + 1 sets.
    pub line_names: *const *const c_char,
    /// Number of repeated line name sets (including both edge lines).
    pub line_name_count: usize,
}

/// Where a grid item starts or ends on one axis. `kind` is a
/// `CrabbyUiGridLineKind`.
/// ```
/// let l = ponko_ffi::crabby_ui_layout_default().grid_column_start;
/// assert_eq!(l.kind, ponko_ffi::CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO as u32);
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
/// let l = ponko_ffi::crabby_ui_layout_default();
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
    /// Optional template components, overriding grid_columns when count is nonzero.
    pub grid_template_columns: *const CrabbyUiTemplateTrack,
    /// Number of column template components.
    pub grid_template_column_count: usize,
    /// Optional template components, overriding grid_rows when count is nonzero.
    pub grid_template_rows: *const CrabbyUiTemplateTrack,
    /// Number of row template components.
    pub grid_template_row_count: usize,
    /// CSS-like area rows, e.g. "header header" and "side main". Copied during the call.
    pub grid_template_areas: *const *const c_char,
    /// Number of area rows. Zero clears the template.
    pub grid_template_area_count: usize,
    /// Space-separated names at each column line; empty strings mean unnamed lines.
    pub grid_column_names: *const *const c_char,
    /// Number of column line name sets (zero means no names).
    pub grid_column_name_count: usize,
    /// Space-separated names at each row line.
    pub grid_row_names: *const *const c_char,
    /// Number of row line name sets.
    pub grid_row_name_count: usize,
    /// Optional area name for all four placement edges. Null leaves numeric placement.
    pub grid_area: *const c_char,
}

/// A node's looks, independent of layout (Rust `UiPaint`).
/// ```
/// let p = ponko_ffi::crabby_ui_paint_default();
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
/// let e = ponko_ffi::CrabbyUiEvent { kind: ponko_ffi::CrabbyUiEventKind::CRABBY_UI_EVENT_CLICKED, id: 3 };
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
/// let i = ponko_ffi::crabby_ui_input_default();
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
/// let l = ponko_ffi::crabby_ui_layout_default();
/// assert_eq!(l.display, ponko_ffi::CrabbyUiDisplay::CRABBY_UI_DISPLAY_FLEX as u32);
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
        grid_template_columns: std::ptr::null(),
        grid_template_column_count: 0,
        grid_template_rows: std::ptr::null(),
        grid_template_row_count: 0,
        grid_template_areas: std::ptr::null(),
        grid_template_area_count: 0,
        grid_column_names: std::ptr::null(),
        grid_column_name_count: 0,
        grid_row_names: std::ptr::null(),
        grid_row_name_count: 0,
        grid_area: std::ptr::null(),
    }
}

/// The default paint (same as Rust's `UiPaint::default()`): the engine's UI
/// colors, the built-in font at 18 units, no border or image.
/// ```
/// let p = ponko_ffi::crabby_ui_paint_default();
/// assert_eq!(p.align, ponko_ffi::CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_START as u32);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_paint_default() -> CrabbyUiPaint {
    paint_to_c(&UiPaint::default())
}

/// Input with nothing pressed: no pointer movement, keys or text.
/// ```
/// assert_eq!(ponko_ffi::crabby_ui_input_default().pointer.x, 0.0);
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
    use ponko::ui_layout::style_helpers as h;
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

// All pointer helpers copy their inputs before returning.
unsafe fn strings(ptr: *const *const c_char, count: usize) -> Result<Vec<String>, Failure> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() || count > u16::MAX as usize {
        return Err(invalid("invalid UI grid string array"));
    }
    // SAFETY: caller supplies count readable pointers to NUL-terminated strings.
    unsafe {
        std::slice::from_raw_parts(ptr, count)
            .iter()
            .map(|&p| {
                if p.is_null() {
                    return Err(invalid("null UI grid string"));
                }
                Ok(text(p)?.to_owned())
            })
            .collect()
    }
}

fn ident(s: &str) -> Result<(), Failure> {
    if s.is_empty()
        || s == "-"
        || matches!(
            s.to_ascii_lowercase().as_str(),
            "auto" | "span" | "default" | "inherit" | "initial" | "unset" | "revert" | "revert-layer"
        )
        || !s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        || s.starts_with(|c: char| c.is_ascii_digit())
        || s.strip_prefix('-')
            .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
    {
        return Err(invalid("invalid UI grid identifier"));
    }
    Ok(())
}

unsafe fn line_names(ptr: *const *const c_char, count: usize) -> Result<Vec<Vec<String>>, Failure> {
    // SAFETY: forwarded caller contract.
    unsafe { strings(ptr, count)? }
        .iter()
        .map(|s| {
            s.split_whitespace()
                .map(|n| {
                    ident(n)?;
                    Ok(n.to_owned())
                })
                .collect()
        })
        .collect()
}

unsafe fn areas(ptr: *const *const c_char, count: usize) -> Result<Option<GridTemplateAreas<String>>, Failure> {
    // SAFETY: forwarded caller contract.
    let rows = unsafe { strings(ptr, count)? };
    if rows.is_empty() {
        return Ok(None);
    }
    let cells: Vec<Vec<&str>> = rows.iter().map(|s| s.split_whitespace().collect()).collect();
    let width = cells[0].len();
    if count >= u16::MAX as usize || width == 0 || width >= u16::MAX as usize || cells.iter().any(|r| r.len() != width)
    {
        return Err(invalid("UI grid area rows must be nonempty and have equal widths"));
    }
    let mut found = std::collections::BTreeMap::<&str, (usize, usize, usize, usize, usize)>::new();
    for (y, row) in cells.iter().enumerate() {
        for (x, &name) in row.iter().enumerate() {
            if name.chars().all(|c| c == '.') {
                continue;
            }
            ident(name)?;
            let b = found.entry(name).or_insert((y, y, x, x, 0));
            b.0 = b.0.min(y);
            b.1 = b.1.max(y);
            b.2 = b.2.min(x);
            b.3 = b.3.max(x);
            b.4 += 1;
        }
    }
    let mut result = Vec::new();
    for (name, (y0, y1, x0, x1, n)) in found {
        if n != (y1 - y0 + 1) * (x1 - x0 + 1) {
            return Err(invalid("UI grid areas must be rectangular"));
        }
        result.push(GridTemplateArea {
            name: name.to_owned(),
            row_start: (y0 + 1) as u16,
            row_end: (y1 + 2) as u16,
            column_start: (x0 + 1) as u16,
            column_end: (x1 + 2) as u16,
        });
    }
    Ok(Some(GridTemplateAreas {
        areas: result,
        row_count: count as u16,
        column_count: width as u16,
    }))
}

fn set_area(style: &mut Style, name: &str) -> Result<(), Failure> {
    let p = if name.is_empty() {
        GridPlacement::Auto
    } else {
        ident(name)?;
        GridPlacement::NamedLine(name.to_owned(), 0)
    };
    style.grid_column = Line {
        start: p.clone(),
        end: p.clone(),
    };
    style.grid_row = Line {
        start: p.clone(),
        end: p,
    };
    Ok(())
}

unsafe fn template_tracks(
    ptr: *const CrabbyUiTemplateTrack,
    count: usize,
) -> Result<Vec<GridTemplateComponent<String>>, Failure> {
    if ptr.is_null() || count > u16::MAX as usize {
        return Err(invalid("invalid UI grid template tracks"));
    }
    let mut result = Vec::new();
    let mut auto = false;
    let mut fixed = true;
    // SAFETY: caller supplies count readable template components and their track arrays.
    for t in unsafe { std::slice::from_raw_parts(ptr, count) } {
        if t.repetition == 0 {
            fixed &= matches!(t.track.kind, 1 | 2);
            result.push(GridTemplateComponent::Single(track(t.track)?));
        } else {
            let repetition = match t.repetition {
                1 if t.count > 0 && t.count <= u16::MAX as u32 => RepetitionCount::Count(t.count as u16),
                2 | 3 if !auto => {
                    auto = true;
                    if t.repetition == 2 {
                        RepetitionCount::AutoFill
                    } else {
                        RepetitionCount::AutoFit
                    }
                }
                _ => {
                    return Err(invalid(
                        "invalid UI grid repetition (only one automatic repeat per axis)",
                    ));
                }
            };
            if t.track_count == 0 || t.track_count > u16::MAX as usize {
                return Err(invalid("UI grid repeat requires tracks"));
            }
            // SAFETY: forwarded caller contract.
            let tracks = unsafe { track_list(t.tracks, t.track_count)? };
            // SAFETY: track_list has validated the pointer, and the caller supplies the array.
            fixed &= unsafe { std::slice::from_raw_parts(t.tracks, t.track_count) }
                .iter()
                .all(|t| matches!(t.kind, 1 | 2));
            if t.line_name_count != 0 && t.line_name_count != t.track_count + 1 {
                return Err(invalid("UI grid repeat line names require track_count + 1 sets"));
            }
            // SAFETY: forwarded readable string-array contract.
            let names = unsafe { line_names(t.line_names, t.line_name_count)? };
            result.push(GridTemplateComponent::Repeat(GridTemplateRepetition {
                count: repetition,
                tracks,
                line_names: names,
            }));
        }
    }
    if auto && !fixed {
        return Err(invalid(
            "UI grid automatic repeats require fixed point or percentage tracks throughout the axis",
        ));
    }
    Ok(result)
}

// Taffy visits repeat line names while iterating outer boundary sets.
fn prepare_grid_names(style: &mut Style) {
    for (tracks, names) in [
        (&style.grid_template_columns, &mut style.grid_template_column_names),
        (&style.grid_template_rows, &mut style.grid_template_row_names),
    ] {
        if tracks
            .iter()
            .any(|t| matches!(t, GridTemplateComponent::Repeat(r) if !r.line_names.is_empty()))
            && names.len() < tracks.len() + 1
        {
            names.resize_with(tracks.len() + 1, Vec::new);
        }
    }
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
/// Descriptor arrays point to their stated counts; strings are NUL-terminated UTF-8.
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
    let mut style = Style {
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
    };
    // SAFETY: all descriptor arrays and strings are covered by the caller contract.
    unsafe {
        if l.grid_template_column_count != 0 {
            style.grid_template_columns = template_tracks(l.grid_template_columns, l.grid_template_column_count)?;
        }
        if l.grid_template_row_count != 0 {
            style.grid_template_rows = template_tracks(l.grid_template_rows, l.grid_template_row_count)?;
        }
        style.grid_template_areas = areas(l.grid_template_areas, l.grid_template_area_count)?;
        style.grid_template_column_names = line_names(l.grid_column_names, l.grid_column_name_count)?;
        style.grid_template_row_names = line_names(l.grid_row_names, l.grid_row_name_count)?;
        if !l.grid_area.is_null() {
            set_area(&mut style, text(l.grid_area)?)?;
        }
    }
    prepare_grid_names(&mut style);
    Ok(style)
}

/// The style for an optional C layout (null is the default).
///
/// # Safety
/// `layout` is null or a readable layout whose arrays and UTF-8 string pointers are valid.
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

/// Fails for a font the engine doesn't know (or that was freed), which
/// would otherwise be laid out in the built-in font without a word.
fn check_font(c: &mut ponko::Ctx, font: ponko::Font) -> Result<(), Failure> {
    if c.gfx.is_live(font) {
        Ok(())
    } else {
        Err(invalid("unknown font"))
    }
}

fn id_to_c(id: Option<UiId>) -> u64 {
    id.map_or(CRABBY_UI_NONE, UiId::bits)
}

/// The UI a `target` names: the screen's, or the one of the world-space panel
/// on an entity.
pub(crate) fn ui_of(c: &mut ponko::Ctx, target: u64) -> Result<&mut Ui, Failure> {
    if target == CRABBY_UI_SCREEN {
        return Ok(&mut c.ui);
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_panel(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, std::ptr::null(), std::ptr::null_mut()) };
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_label(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, c"Hi".as_ptr(), std::ptr::null(), std::ptr::null_mut()) };
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_button(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, c"Buy".as_ptr(), std::ptr::null(), std::ptr::null_mut()) };
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
/// let t = ponko_ffi::CrabbyTexture { id: 0, width: 1, height: 1 };
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_image(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, t, std::ptr::null(), std::ptr::null_mut()) };
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_scroll(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, std::ptr::null(), std::ptr::null_mut()) };
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_text_input(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) };
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_toggle(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, std::ptr::null(), true, std::ptr::null(), std::ptr::null_mut()) };
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
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_slider(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, ponko_ffi::CRABBY_UI_NONE, 0.5, std::ptr::null(), std::ptr::null_mut()) };
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_remove(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_layout(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `layout` null or readable, with valid descriptor arrays and NUL-terminated UTF-8 strings.
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

unsafe fn update_style(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    f: impl FnOnce(&mut Style) -> Result<(), Failure>,
) -> Result<(), Failure> {
    // SAFETY: forwarded context contract.
    unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            let mut style = ui
                .layout_style(id)
                .ok_or_else(|| invalid("UI node has no style"))?
                .clone();
            f(&mut style)?;
            prepare_grid_names(&mut style);
            ui.set_layout(id, style);
            Ok(())
        })
    }
}

/// Replaces area rows; count zero clears them. Invalid/non-rectangular templates leave the layout unchanged.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_grid_template_areas(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null(), 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` is null or live. All string pointers are readable NUL-terminated UTF-8;
/// arrays contain `count` readable string pointers. Inputs are copied during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_template_areas(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    rows: *const *const c_char,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let value = areas(rows, count)?;
        update_style(ctx, target, id, |s| {
            s.grid_template_areas = value;
            Ok(())
        })
    })
}

/// Replaces space-separated line name sets on an axis (0 columns, 1 rows). Empty strings denote unnamed lines. With repeats, sets correspond to template component boundaries.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_grid_line_names(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, 0, std::ptr::null(), 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` is null or live. All string pointers are readable NUL-terminated UTF-8;
/// arrays contain `count` readable string pointers. Inputs are copied during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_line_names(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    axis: u32,
    names: *const *const c_char,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let value = line_names(names, count)?;
        update_style(ctx, target, id, |s| {
            match axis {
                0 => s.grid_template_column_names = value,
                1 => s.grid_template_row_names = value,
                _ => return Err(invalid("UI grid axis must be 0 or 1")),
            };
            Ok(())
        })
    })
}

/// Places all four edges by an area name. Empty string restores automatic placement. Unknown names follow taffy implicit-line resolution.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_grid_area(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` is null or live. All string pointers are readable NUL-terminated UTF-8;
/// arrays contain `count` readable string pointers. Inputs are copied during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_area(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let name = text(name)?;
        update_style(ctx, target, id, |s| set_area(s, name))
    })
}

/// Places one named edge: axis 0 columns/1 rows; end false start/true end. Occurrence 0 uses area-aware resolution, positive/negative count matching named lines.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_grid_named_line(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, 0, false, std::ptr::null(), 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` is null or live. All string pointers are readable NUL-terminated UTF-8;
/// arrays contain `count` readable string pointers. Inputs are copied during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_named_line(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    axis: u32,
    end: bool,
    name: *const c_char,
    occurrence: i32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let name = text(name)?;
        ident(name)?;
        let occurrence = i16::try_from(occurrence).map_err(|_| invalid("UI grid line occurrence out of range"))?;
        update_style(ctx, target, id, |s| {
            let line = match axis {
                0 => &mut s.grid_column,
                1 => &mut s.grid_row,
                _ => return Err(invalid("UI grid axis must be 0 or 1")),
            };
            let p = GridPlacement::NamedLine(name.to_owned(), occurrence);
            if end {
                line.end = p;
            } else {
                line.start = p;
            }
            Ok(())
        })
    })
}

/// Replaces a node's paint. A null `paint` restores the default. Fonts must
/// come from `crabby_ctx_load_font`; an unknown one is an error.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_paint(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null()) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_text(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, c"x".as_ptr()) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_text(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null_mut(), 0, &mut 0) } as u32, 1);
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
        // Outputs first: a bad output is refused before the tree is touched,
        // whatever the node.
        check_output(buffer, capacity, out_len)?;
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            copy_string(ui.text(id).unwrap_or(""), buffer, capacity, out_len)
        })
    })
}

/// Enables or disables a node and everything under it. Disabled controls take
/// no input and draw with the paint's `disabled` color.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_enabled(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, false) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_value(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, 0.5) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_value(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, &mut 0.0) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_scroll(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, ponko_ffi::CrabbyVec2 { x: 0.0, y: 40.0 }) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_scroll(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, &mut Default::default()) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_push_screen(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, true) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_pop_screen(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_top_screen(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_focus(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_scale(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 1.25) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_rect(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, &mut Default::default()) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_hit_test(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, Default::default(), &mut 0) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_wants_pointer(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut false) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_wants_keyboard(std::ptr::null_mut(), &mut false) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_wanted` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_wants_keyboard(ctx: *mut CrabbyCtx, out_wanted: *mut bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_wanted, c.ui.wants_keyboard())) })
}

// ---- events and frames -------------------------------------------------------

/// Takes the oldest queued event into `out_event`, or fails with
/// `CRABBY_NOT_FOUND` when none are left. Unread events expire at the next UI
/// frame, so drain them every frame. Removing a node drops its events.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_next_event(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut ponko_ffi::CrabbyUiEvent { kind: Default::default(), id: 0 }) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_layout(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, Default::default()) } as u32, 1);
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_frame(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, Default::default(), std::ptr::null()) } as u32, 1);
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
            Some(i) => ponko::ui::UiInput {
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
            None => ponko::ui::UiInput::default(),
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
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_insert_text(std::ptr::null_mut(), c"Hi".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_insert_text(ctx: *mut CrabbyCtx, text: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let value = crate::ctx::text(text)?;
        ctx_access(ctx, |c| {
            c.input.insert_text(value);
            Ok(())
        })
    })
}

// ---- layout presets (Rust `UiLayout`) ------------------------------------------

const fn points(value: f32) -> CrabbyUiLength {
    length(CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS, value)
}
const fn fraction(value: f32) -> CrabbyUiLength {
    length(CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT, value)
}
fn column_layout() -> CrabbyUiLayout {
    CrabbyUiLayout {
        flex_direction: CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN as u32,
        ..crabby_ui_layout_default()
    }
}

/// A full-width row `height` units tall that keeps its height; its children
/// go left to right (Rust `UiLayout::row`). Change `gap`, `padding` or
/// `flex_grow` on the result as needed.
/// ```
/// let l = ponko_ffi::crabby_ui_layout_row(30.0);
/// assert_eq!((l.height.value, l.width.value, l.flex_shrink), (30.0, 1.0, 0.0));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_row(height: f32) -> CrabbyUiLayout {
    CrabbyUiLayout {
        width: fraction(1.0),
        height: points(height),
        flex_shrink: 0.0,
        ..crabby_ui_layout_default()
    }
}

/// A full-height column `width` units wide that keeps its width; its
/// children stack top to bottom (Rust `UiLayout::column`).
/// ```
/// let l = ponko_ffi::crabby_ui_layout_column(200.0);
/// assert_eq!((l.width.value, l.height.value), (200.0, 1.0));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_column(width: f32) -> CrabbyUiLayout {
    CrabbyUiLayout {
        width: points(width),
        height: fraction(1.0),
        flex_shrink: 0.0,
        ..column_layout()
    }
}

/// All of the parent (the window, for a screen root), shrinking to the
/// space its siblings leave; children stack top to bottom (Rust
/// `UiLayout::fill`).
/// ```
/// let l = ponko_ffi::crabby_ui_layout_fill();
/// assert_eq!((l.width.value, l.height.value), (1.0, 1.0));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_fill() -> CrabbyUiLayout {
    CrabbyUiLayout {
        width: fraction(1.0),
        height: fraction(1.0),
        min_width: points(0.0),
        min_height: points(0.0),
        flex_shrink: 1.0,
        ..column_layout()
    }
}

/// Exactly `width` x `height` units, never shrunk; children stack top to
/// bottom (Rust `UiLayout::fixed`).
/// ```
/// let l = ponko_ffi::crabby_ui_layout_fixed(220.0, 40.0);
/// assert_eq!((l.width.value, l.height.value, l.flex_shrink), (220.0, 40.0, 0.0));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_fixed(width: f32, height: f32) -> CrabbyUiLayout {
    CrabbyUiLayout {
        width: points(width),
        height: points(height),
        flex_shrink: 0.0,
        ..column_layout()
    }
}

/// Out of the flow, at `x`, `y` and `width` x `height` in fractions of the
/// parent (of the window, for a screen root): a minimap in a corner, a HUD
/// strip (Rust `UiLayout::anchored`).
/// ```
/// let l = ponko_ffi::crabby_ui_layout_anchored(0.75, 0.0, 0.25, 0.25);
/// assert_eq!((l.inset.left.value, l.width.value), (0.75, 0.25));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_anchored(x: f32, y: f32, width: f32, height: f32) -> CrabbyUiLayout {
    CrabbyUiLayout {
        position: CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE as u32,
        inset: CrabbyUiSides {
            top: fraction(y),
            right: AUTO,
            bottom: AUTO,
            left: fraction(x),
        },
        width: fraction(width),
        height: fraction(height),
        ..column_layout()
    }
}

/// `width` x `height` units in the middle of the parent (of the window, for
/// a screen root): a dialog (Rust `UiLayout::centered`). To move it to a
/// distance from the top, set `inset.top` to that many points and
/// `margin.top` to zero.
/// ```
/// let l = ponko_ffi::crabby_ui_layout_centered(360.0, 180.0);
/// assert_eq!((l.inset.left.value, l.margin.left.value), (0.5, -180.0));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_centered(width: f32, height: f32) -> CrabbyUiLayout {
    CrabbyUiLayout {
        position: CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE as u32,
        inset: CrabbyUiSides {
            top: fraction(0.5),
            right: AUTO,
            bottom: AUTO,
            left: fraction(0.5),
        },
        margin: CrabbyUiSides {
            top: points(-height * 0.5),
            right: AUTO,
            bottom: AUTO,
            left: points(-width * 0.5),
        },
        width: points(width),
        height: points(height),
        ..column_layout()
    }
}

// ---- theme (Rust `UiTheme`) ----------------------------------------------------

/// The colors and text defaults of a UI tree (Rust `UiTheme`). Every new
/// node takes its paint from it; paint set on a node overrides it. The screen
/// UI's immediate widgets (`crabby_ctx_ui_style`) follow it too. Start from
/// `crabby_ui_theme_default()` or `crabby_ctx_ui_theme`.
/// ```
/// let t = ponko_ffi::crabby_ui_theme_default();
/// assert!(t.title_size > t.text_size);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiTheme {
    /// Background of panels and scroll areas.
    pub panel: CrabbyColor,
    /// Background of controls: buttons, inputs, toggles, sliders.
    pub control: CrabbyColor,
    /// Control background while hovered or focused.
    pub hover: CrabbyColor,
    /// Control background while held.
    pub pressed: CrabbyColor,
    /// Background while disabled.
    pub disabled: CrabbyColor,
    /// Borders, focus ring, slider fill, checked toggles.
    pub accent: CrabbyColor,
    /// Text.
    pub text: CrabbyColor,
    /// Text of titles (`crabby_ctx_ui_add_title`).
    pub title: CrabbyColor,
    /// Font, from `crabby_ctx_load_font`; `{0}` is the built-in font.
    pub font: CrabbyFont,
    /// Line height of text in logical units.
    pub text_size: f32,
    /// Line height of titles in logical units.
    pub title_size: f32,
    /// Border thickness of controls in logical units.
    pub border_width: f32,
}

fn theme_to_c(t: &UiTheme) -> CrabbyUiTheme {
    CrabbyUiTheme {
        panel: to_c(t.panel),
        control: to_c(t.control),
        hover: to_c(t.hover),
        pressed: to_c(t.pressed),
        disabled: to_c(t.disabled),
        accent: to_c(t.accent),
        text: to_c(t.text),
        title: to_c(t.title),
        font: font_to_c(t.font),
        text_size: t.text_size,
        title_size: t.title_size,
        border_width: t.border_width,
    }
}

fn theme_from_c(t: &CrabbyUiTheme) -> Result<UiTheme, Failure> {
    let (text_size, title_size, border_width) = (num(t.text_size)?, num(t.title_size)?, num(t.border_width)?);
    if text_size < 0.0 || title_size < 0.0 || border_width < 0.0 {
        return Err(invalid("UI text sizes and border width must not be negative"));
    }
    Ok(UiTheme {
        panel: to_c(t.panel),
        control: to_c(t.control),
        hover: to_c(t.hover),
        pressed: to_c(t.pressed),
        disabled: to_c(t.disabled),
        accent: to_c(t.accent),
        text: to_c(t.text),
        title: to_c(t.title),
        font: font_from_c(t.font),
        text_size,
        title_size,
        border_width,
    })
}

/// The engine's default theme (Rust `UiTheme::default()`).
/// ```
/// assert_eq!(ponko_ffi::crabby_ui_theme_default().font.id, 0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_theme_default() -> CrabbyUiTheme {
    theme_to_c(&UiTheme::default())
}

/// Writes the theme of a UI to `out_theme`.
/// ```
/// let mut t = ponko_ffi::crabby_ui_theme_default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_theme(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut t) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_theme` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_theme(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_theme: *mut CrabbyUiTheme,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { with_ui(ctx, target, |ui| out(out_theme, theme_to_c(ui.theme()))) })
}

/// Replaces the theme of a UI. New nodes take their paint from it; existing
/// nodes follow it in every paint field still equal to the old theme's value
/// and keep the fields set on them. On the screen's UI the immediate widgets'
/// style follows the same way. Fonts must come from `crabby_ctx_load_font`.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_theme(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `theme` readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_theme(
    ctx: *mut CrabbyCtx,
    target: u64,
    theme: *const CrabbyUiTheme,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let Some(theme) = theme.as_ref() else {
            return Err(invalid("theme is null"));
        };
        let theme = theme_from_c(theme)?;
        ctx_access(ctx, |c| {
            check_font(c, theme.font)?;
            ui_of(c, target)?.set_theme(theme);
            Ok(())
        })
    })
}

/// Writes a node's resolved paint (its theme's paint plus what was set on
/// it) to `out_paint`.
/// ```
/// let mut p = ponko_ffi::crabby_ui_paint_default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_paint(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, &mut p) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_paint` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_paint(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_paint: *mut CrabbyUiPaint,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            out(out_paint, paint_to_c(&ui.paint(id).unwrap_or_default()))
        })
    })
}

/// Adds a title under `parent`: a label in the theme's title color and size,
/// which it keeps following when the theme changes (Rust `Ui::title`). A
/// title needs a parent. See `crabby_ctx_ui_add_panel` for `layout` and
/// `out_id`.
/// ```
/// let status = unsafe { ponko_ffi::crabby_ctx_ui_add_title(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, c"Shop".as_ptr(), std::ptr::null(), std::ptr::null_mut()) };
/// assert_eq!(status as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `text` null or NUL-terminated; `layout` null or readable; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_title(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        if out_id.is_null() {
            return Err(invalid("output is null"));
        }
        let (text, style) = (optional_text(text)?, style_or_default(layout)?);
        with_ui(ctx, target, |ui| {
            if parent == CRABBY_UI_NONE {
                return Err(invalid("a title needs a parent"));
            }
            let parent = existing(ui, parent)?;
            out(out_id, ui.title(parent, text).layout(style).id().bits())
        })
    })
}

// ---- names and clicks ----------------------------------------------------------

/// Names a node so code that didn't build it finds it with
/// `crabby_ctx_ui_find` (Rust `Ui::set_name`). A name belongs to one node of
/// a UI: naming another node the same moves it there. An empty or null name
/// takes the node's name off.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_set_name(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, c"shop.buy".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `name` null or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_name(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let name = optional_text(name)?;
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            ui.set_name(id, name);
            Ok(())
        })
    })
}

/// Writes the node named `name` to `out_id`, or `CRABBY_UI_NONE` when no
/// node has that name (Rust `Ui::find`).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_find(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, c"shop.buy".as_ptr(), &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `name` NUL-terminated; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_find(
    ctx: *mut CrabbyCtx,
    target: u64,
    name: *const c_char,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let name = text(name)?;
        with_ui(ctx, target, |ui| out(out_id, id_to_c(ui.find(name))))
    })
}

/// Reads a node's name into `buffer` the way `crabby_ctx_ui_text` reads its
/// text; a node without a name gives an empty string.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_name(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, 0, std::ptr::null_mut(), 0, &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `buffer` writable for `capacity` bytes (or null with
/// capacity 0); `out_len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_name(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        check_output(buffer, capacity, out_len)?;
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            copy_string(ui.name(id).unwrap_or(""), buffer, capacity, out_len)
        })
    })
}

/// Takes the queued events and writes the node of the last click among them
/// to `out_id`, or `CRABBY_UI_NONE` without one (Rust `Ui::take_click`). For
/// screens that react only to clicks: the other events are dropped, so use
/// `crabby_ctx_ui_next_event` on a screen whose inputs matter.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_take_click(std::ptr::null_mut(), ponko_ffi::CRABBY_UI_SCREEN, &mut 0) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_id` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_take_click(ctx: *mut CrabbyCtx, target: u64, out_id: *mut u64) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        if out_id.is_null() {
            return Err(invalid("output is null"));
        }
        with_ui(ctx, target, |ui| out(out_id, id_to_c(ui.take_click())))
    })
}

/// Refuses a null `out_len`, or a null `buffer` with a capacity.
fn check_output(buffer: *mut c_char, capacity: usize, out_len: *mut usize) -> Result<(), Failure> {
    if out_len.is_null() || (buffer.is_null() && capacity != 0) {
        return Err(invalid("output is null"));
    }
    Ok(())
}

/// Writes `value` NUL-terminated into `buffer` of `capacity` bytes and its
/// length to `out_len` (see `crabby_ctx_ui_text`).
///
/// # Safety
/// `buffer` writable for `capacity` bytes, or null with capacity 0; `out_len` null or writable.
unsafe fn copy_string(value: &str, buffer: *mut c_char, capacity: usize, out_len: *mut usize) -> Result<(), Failure> {
    check_output(buffer, capacity, out_len)?;
    // SAFETY: validated non-null outputs; the caller supplies the stated capacity.
    unsafe {
        out_len.write(value.len());
        if value.len() < capacity {
            std::ptr::copy_nonoverlapping(value.as_ptr(), buffer.cast(), value.len());
            buffer.add(value.len()).write(0);
        } else if capacity != 0 {
            buffer.write(0);
        }
    }
    Ok(())
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
    fn layout_presets_are_the_rust_presets() {
        use ponko::ui::UiLayout;
        let pairs = [
            (crabby_ui_layout_row(20.0), UiLayout::row(20.0)),
            (crabby_ui_layout_column(150.0), UiLayout::column(150.0)),
            (crabby_ui_layout_fill(), UiLayout::fill()),
            (crabby_ui_layout_fixed(220.0, 40.0), UiLayout::fixed(220.0, 40.0)),
            (
                crabby_ui_layout_anchored(0.1, 0.2, 0.3, 0.4),
                UiLayout::anchored(0.1, 0.2, 0.3, 0.4),
            ),
            (
                crabby_ui_layout_centered(360.0, 180.0),
                UiLayout::centered(360.0, 180.0),
            ),
        ];
        for (c, rust) in pairs {
            // SAFETY: presets hold no pointers.
            let style = unsafe { layout_style(&c) }.ok().unwrap();
            assert_eq!(style, rust.into_style());
        }
    }

    #[test]
    fn themes_round_trip_and_reject_negative_sizes() {
        let theme = UiTheme {
            accent: Color::GREEN,
            title_size: 40.0,
            ..UiTheme::default()
        };
        assert_eq!(theme_from_c(&theme_to_c(&theme)).ok(), Some(theme));
        let bad = CrabbyUiTheme {
            text_size: -1.0,
            ..crabby_ui_theme_default()
        };
        assert!(theme_from_c(&bad).is_err());
    }

    #[test]
    fn ids_round_trip_through_integers() {
        assert_eq!(UiId::from_bits(UiId::from_bits(12).bits()).bits(), 12);
        assert_eq!(id_to_c(None), CRABBY_UI_NONE);
    }
}
