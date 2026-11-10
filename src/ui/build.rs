//! Builders, layout presets and a theme for the retained [`Ui`].
//!
//! Everything here is a thin layer over [`Ui::add_child`] and the setters: a
//! builder adds the node first, each method changes it right away, and
//! [`UiBuilder::id`] hands back its [`UiId`]. Nothing new is stored in the
//! tree except the theme, which gives new nodes their [`UiPaint`].

use super::retained::{Ui, UiId, UiPaint, UiTextAlign, UiWidget};
use crate::{Color, Font, Texture};
use taffy::prelude::*;

/// Layout presets for the common shapes of game UI, so a screen needs no
/// hand-written `ui_layout::Style` helpers. Each preset converts into a
/// `ui_layout::Style`; [`UiLayout::custom`] keeps Taffy's full power.
///
/// Lengths are logical window units. Fractions are of the parent's size (of
/// the window for a screen root). Children stack top to bottom, except
/// inside a [`UiLayout::row`], where they go left to right.
/// ```
/// use ponko::*;
/// let mut ui = Ui::default();
/// let root = ui.root().layout(UiLayout::fill().padding(10.0).gap(4.0)).id();
/// let a = ui.label(root, "A").layout(UiLayout::row(20.0)).id();
/// let b = ui.label(root, "B").layout(UiLayout::row(20.0)).id();
/// ui.push_screen(root, false);
/// ui.layout(vec2(200.0, 100.0));
/// assert_eq!(ui.rect(a).unwrap().min, vec2(10.0, 10.0));
/// assert_eq!(ui.rect(b).unwrap().min, vec2(10.0, 34.0));
/// assert_eq!(ui.rect(b).unwrap().size(), vec2(180.0, 20.0));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UiLayout(Style);

impl Default for UiLayout {
    fn default() -> Self {
        Self::auto()
    }
}

impl From<UiLayout> for Style {
    fn from(layout: UiLayout) -> Self {
        layout.0
    }
}

fn column_style() -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        ..Default::default()
    }
}

impl UiLayout {
    /// Sized by its content (text, children); Taffy's default style.
    /// ```
    /// use ponko::*;
    /// let style: ui_layout::Style = UiLayout::auto().into();
    /// assert_eq!(style, ui_layout::Style::default());
    /// ```
    pub fn auto() -> Self {
        Self(Style::default())
    }
    /// Any Taffy style, for flex and grid settings the presets don't cover.
    /// ```
    /// use ponko::*;
    /// let layout = UiLayout::custom(ui_layout::Style { flex_grow: 2.0, ..Default::default() });
    /// assert_eq!(layout.style().flex_grow, 2.0);
    /// ```
    pub fn custom(style: Style) -> Self {
        Self(style)
    }
    /// A horizontal strip: the parent's full width, `height` tall, never
    /// squeezed. Its children go left to right.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::row(30.0).into_style();
    /// assert_eq!(style.flex_direction, ui_layout::FlexDirection::Row);
    /// ```
    pub fn row(height: f32) -> Self {
        Self(Style {
            flex_direction: FlexDirection::Row,
            size: Size {
                width: percent(1.0),
                height: length(height),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })
    }
    /// A vertical strip: `width` wide, the parent's full height, never
    /// squeezed. Its children go top to bottom.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fill().horizontal()).id();
    /// let side = ui.panel(root).layout(UiLayout::column(120.0)).id();
    /// ui.push_screen(root, false);
    /// ui.layout(vec2(400.0, 300.0));
    /// assert_eq!(ui.rect(side).unwrap().size(), vec2(120.0, 300.0));
    /// ```
    pub fn column(width: f32) -> Self {
        Self(Style {
            size: Size {
                width: length(width),
                height: percent(1.0),
            },
            flex_shrink: 0.0,
            ..column_style()
        })
    }
    /// The parent's whole content box. Next to siblings in a column it
    /// shrinks to the space they leave (it can shrink to nothing).
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fill()).id();
    /// let header = ui.label(root, "Inventory").layout(UiLayout::row(40.0)).id();
    /// let list = ui.scroll_area(root).layout(UiLayout::fill()).id();
    /// ui.push_screen(root, false);
    /// ui.layout(vec2(400.0, 300.0));
    /// assert_eq!(ui.rect(list).unwrap().size(), vec2(400.0, 260.0));
    /// ```
    pub fn fill() -> Self {
        Self(Style {
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            min_size: Size::length(0.0),
            flex_shrink: 1.0,
            ..column_style()
        })
    }
    /// Exactly `width` x `height`, never squeezed.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::fixed(200.0, 40.0).into_style();
    /// assert_eq!(style.flex_shrink, 0.0);
    /// ```
    pub fn fixed(width: f32, height: f32) -> Self {
        Self(Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            flex_shrink: 0.0,
            ..column_style()
        })
    }
    /// Placed by fractions of the parent, out of the flow of its siblings:
    /// top-left corner at (`x`, `y`), size `width` x `height`. For a screen
    /// root these are fractions of the window, so a HUD keeps its place at
    /// any resolution.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let hud = ui.root().layout(UiLayout::anchored(0.75, 0.0, 0.25, 0.1)).id();
    /// ui.push_screen(hud, false);
    /// ui.layout(vec2(800.0, 600.0));
    /// assert_eq!(ui.rect(hud).unwrap().min, vec2(600.0, 0.0));
    /// assert_eq!(ui.rect(hud).unwrap().size(), vec2(200.0, 60.0));
    /// ```
    pub fn anchored(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self(Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: percent(x),
                top: percent(y),
                right: auto(),
                bottom: auto(),
            },
            size: Size {
                width: percent(width),
                height: percent(height),
            },
            ..column_style()
        })
    }
    /// `width` x `height`, centered in the parent and out of the flow of its
    /// siblings: a dialog.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let dialog = ui.root().layout(UiLayout::centered(200.0, 100.0)).id();
    /// ui.push_screen(dialog, true);
    /// ui.layout(vec2(800.0, 600.0));
    /// assert_eq!(ui.rect(dialog).unwrap().min, vec2(300.0, 250.0));
    /// ```
    pub fn centered(width: f32, height: f32) -> Self {
        Self(Style {
            position: Position::Absolute,
            inset: taffy::Rect {
                left: percent(0.5),
                top: percent(0.5),
                right: auto(),
                bottom: auto(),
            },
            margin: taffy::Rect {
                left: length(-width * 0.5),
                top: length(-height * 0.5),
                right: auto(),
                bottom: auto(),
            },
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..column_style()
        })
    }
    /// A grid of `columns` equal columns as wide as the parent, with rows
    /// `row_height` tall and as many as the children need: an inventory.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fixed(300.0, 300.0)).id();
    /// let grid = ui.panel(root).layout(UiLayout::grid(3, 50.0)).id();
    /// let cells: Vec<UiId> = (0..4).map(|_| ui.button(grid, "").id()).collect();
    /// ui.push_screen(root, false);
    /// ui.layout(vec2(800.0, 600.0));
    /// assert_eq!(ui.rect(cells[1]).unwrap().min, vec2(100.0, 0.0));
    /// assert_eq!(ui.rect(cells[3]).unwrap().min, vec2(0.0, 50.0));
    /// ```
    pub fn grid(columns: usize, row_height: f32) -> Self {
        Self(Style {
            display: Display::Grid,
            size: Size {
                width: percent(1.0),
                height: auto(),
            },
            grid_template_columns: vec![fr(1.0); columns],
            grid_auto_rows: vec![length(row_height)],
            flex_shrink: 0.0,
            ..Default::default()
        })
    }

    /// Space between children, in both directions.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::fill().gap(8.0).into_style();
    /// assert_eq!(style.gap, ui_layout::Size::length(8.0));
    /// ```
    pub fn gap(mut self, gap: f32) -> Self {
        self.0.gap = Size::length(gap);
        self
    }
    /// Space inside the edges, on all four sides.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::fill().padding(12.0).into_style();
    /// assert_eq!(style.padding, ui_layout::Rect::length(12.0));
    /// ```
    pub fn padding(mut self, padding: f32) -> Self {
        self.0.padding = taffy::Rect::length(padding);
        self
    }
    /// Take a `factor` share of the space the siblings leave free, along the
    /// parent's direction.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fixed(300.0, 100.0)).id();
    /// let top = ui.panel(root).layout(UiLayout::auto().grow(1.0)).id();
    /// ui.panel(root).layout(UiLayout::row(40.0));
    /// ui.push_screen(root, false);
    /// ui.layout(vec2(800.0, 600.0));
    /// assert_eq!(ui.rect(top).unwrap().size().y, 60.0);
    /// ```
    pub fn grow(mut self, factor: f32) -> Self {
        self.0.flex_grow = factor;
        self
    }
    /// Children go left to right.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::fill().horizontal().into_style();
    /// assert_eq!(style.flex_direction, ui_layout::FlexDirection::Row);
    /// ```
    pub fn horizontal(mut self) -> Self {
        self.0.flex_direction = FlexDirection::Row;
        self
    }
    /// Children go top to bottom.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::row(30.0).vertical().into_style();
    /// assert_eq!(style.flex_direction, ui_layout::FlexDirection::Column);
    /// ```
    pub fn vertical(mut self) -> Self {
        self.0.flex_direction = FlexDirection::Column;
        self
    }
    /// Width in logical units.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::row(30.0).width(200.0).into_style();
    /// assert_eq!(style.size.width, ui_layout::Dimension::length(200.0));
    /// ```
    pub fn width(mut self, width: f32) -> Self {
        self.0.size.width = length(width);
        self
    }
    /// Height in logical units.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::column(30.0).height(200.0).into_style();
    /// assert_eq!(style.size.height, ui_layout::Dimension::length(200.0));
    /// ```
    pub fn height(mut self, height: f32) -> Self {
        self.0.size.height = length(height);
        self
    }
    /// Width as a fraction of the parent's.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::grid(3, 50.0).width_fraction(0.95).into_style();
    /// assert_eq!(style.size.width, ui_layout::Dimension::percent(0.95));
    /// ```
    pub fn width_fraction(mut self, fraction: f32) -> Self {
        self.0.size.width = percent(fraction);
        self
    }
    /// Height as a fraction of the parent's.
    /// ```
    /// use ponko::*;
    /// let style = UiLayout::row(30.0).height_fraction(0.5).into_style();
    /// assert_eq!(style.size.height, ui_layout::Dimension::percent(0.5));
    /// ```
    pub fn height_fraction(mut self, fraction: f32) -> Self {
        self.0.size.height = percent(fraction);
        self
    }
    /// The Taffy style, to read or adjust a field the presets don't cover.
    /// ```
    /// use ponko::*;
    /// let mut layout = UiLayout::fill();
    /// layout.style_mut().flex_wrap = ui_layout::FlexWrap::Wrap;
    /// assert_eq!(layout.style().flex_wrap, ui_layout::FlexWrap::Wrap);
    /// ```
    pub fn style(&self) -> &Style {
        &self.0
    }
    /// Mutable access to the Taffy style; see [`UiLayout::style`].
    /// ```
    /// use ponko::*;
    /// let mut layout = UiLayout::auto();
    /// layout.style_mut().flex_grow = 1.0;
    /// ```
    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.0
    }
    /// The finished Taffy style, for [`Ui::add`] and [`Ui::set_layout`].
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let id = ui.add(UiWidget::Panel, UiLayout::fill().into_style());
    /// ```
    pub fn into_style(self) -> Style {
        self.0
    }
}

/// Colors and text defaults shared by the whole tree. Every new node takes
/// its [`UiPaint`] from the theme for its widget; paint set on a node
/// (through a builder or [`Ui::set_paint`]) overrides it.
/// ```
/// use ponko::*;
/// let mut ui = Ui::default();
/// ui.set_theme(UiTheme { text: Color::YELLOW, text_size: 14.0, ..Default::default() });
/// let root = ui.root().id();
/// let label = ui.label(root, "Gold").id();
/// assert_eq!(ui.paint(label).unwrap().color, Color::YELLOW);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiTheme {
    /// Background of panels and scroll areas.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().panel;
    /// ```
    pub panel: Color,
    /// Background of buttons, text inputs, toggles and sliders.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().control;
    /// ```
    pub control: Color,
    /// Hovered or focused background.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().hover;
    /// ```
    pub hover: Color,
    /// Pointer-held background.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().pressed;
    /// ```
    pub pressed: Color,
    /// Disabled background.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().disabled;
    /// ```
    pub disabled: Color,
    /// Borders, the focus ring and slider fill.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().accent;
    /// ```
    pub accent: Color,
    /// Text and image tint.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().text;
    /// ```
    pub text: Color,
    /// Font handle, from `Graphics::load_font`.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().font;
    /// ```
    pub font: Font,
    /// Text line height in logical units.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().text_size;
    /// ```
    pub text_size: f32,
    /// Border thickness of controls in logical units; panels and text have none.
    /// ```
    /// use ponko::*;
    /// let _ = UiTheme::default().border_width;
    /// ```
    pub border_width: f32,
}

impl Default for UiTheme {
    /// The engine's look: dark blue-grey controls, white text, orange accent.
    fn default() -> Self {
        let paint = UiPaint::default();
        Self {
            panel: paint.background,
            control: paint.background,
            hover: paint.hover,
            pressed: paint.pressed,
            disabled: paint.disabled,
            accent: paint.border,
            text: paint.color,
            font: paint.font,
            text_size: paint.font_size,
            border_width: paint.border_width,
        }
    }
}

impl UiTheme {
    /// The paint a new node of this widget gets. Labels and images have a
    /// clear background.
    /// ```
    /// use ponko::*;
    /// let theme = UiTheme::default();
    /// assert_eq!(theme.paint(UiWidget::Label).background.a, 0.0);
    /// assert_eq!(theme.paint(UiWidget::Button).background, theme.control);
    /// ```
    pub fn paint(&self, widget: UiWidget) -> UiPaint {
        let (background, border_width) = match widget {
            UiWidget::Label | UiWidget::Image => (Color::rgba(0.0, 0.0, 0.0, 0.0), 0.0),
            UiWidget::Panel | UiWidget::Scroll => (self.panel, 0.0),
            UiWidget::Button | UiWidget::TextInput | UiWidget::Toggle | UiWidget::Slider => {
                (self.control, self.border_width)
            }
        };
        UiPaint {
            background,
            hover: self.hover,
            pressed: self.pressed,
            disabled: self.disabled,
            border: self.accent,
            border_width,
            color: self.text,
            font: self.font,
            font_size: self.text_size,
            ..UiPaint::default()
        }
    }
}

/// A node being built: each method changes it right away, and [`UiBuilder::id`]
/// returns its id. Dropping the builder keeps the node, so a label nothing
/// refers to again needs no `.id()`.
/// ```
/// use ponko::*;
/// let mut ui = Ui::default();
/// let root = ui.root().layout(UiLayout::fill()).id();
/// let title = ui.label(root, "LOBBY").layout(UiLayout::row(20.0)).size(13.0).color(Color::YELLOW).id();
/// assert_eq!(ui.text(title), Some("LOBBY"));
/// assert_eq!(ui.paint(title).unwrap().font_size, 13.0);
/// ```
pub struct UiBuilder<'a> {
    ui: &'a mut Ui,
    id: UiId,
}

impl UiBuilder<'_> {
    fn paint_with(self, f: impl FnOnce(&mut UiPaint)) -> Self {
        if let Some(paint) = self.ui.paint_mut(self.id) {
            f(paint);
        }
        self
    }
    /// The node's id.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// assert!(ui.rect(root).is_some());
    /// ```
    pub fn id(self) -> UiId {
        self.id
    }
    /// Layout: a [`UiLayout`] preset or any `ui_layout::Style`.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fixed(100.0, 50.0)).id();
    /// assert_eq!(ui.layout_style(root), Some(&UiLayout::fixed(100.0, 50.0).into_style()));
    /// ```
    pub fn layout(self, layout: impl Into<Style>) -> Self {
        self.ui.set_layout(self.id, layout.into());
        self
    }
    /// Label, button or input text; see [`Ui::set_text`].
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let name = ui.text_input(root).text("Player").id();
    /// assert_eq!(ui.text(name), Some("Player"));
    /// ```
    pub fn text(self, text: impl AsRef<str>) -> Self {
        self.ui.set_text(self.id, text);
        self
    }
    /// Text line height in logical units.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let id = ui.label(root, "Hi").size(30.0).id();
    /// assert_eq!(ui.paint(id).unwrap().font_size, 30.0);
    /// ```
    pub fn size(self, font_size: f32) -> Self {
        self.paint_with(|p| p.font_size = font_size)
    }
    /// Text and image tint.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let id = ui.label(root, "Hi").color(Color::RED).id();
    /// assert_eq!(ui.paint(id).unwrap().color, Color::RED);
    /// ```
    pub fn color(self, color: Color) -> Self {
        self.paint_with(|p| p.color = color)
    }
    /// Normal background.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().background(Color::BLACK).id();
    /// assert_eq!(ui.paint(root).unwrap().background, Color::BLACK);
    /// ```
    pub fn background(self, color: Color) -> Self {
        self.paint_with(|p| p.background = color)
    }
    /// Border color and thickness in logical units.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().border(Color::WHITE, 2.0).id();
    /// assert_eq!(ui.paint(root).unwrap().border_width, 2.0);
    /// ```
    pub fn border(self, color: Color, width: f32) -> Self {
        self.paint_with(|p| {
            p.border = color;
            p.border_width = width;
        })
    }
    /// Border thickness in logical units, in the theme's accent color.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let slot = ui.button(root, "").border_width(1.0).id();
    /// assert_eq!(ui.paint(slot).unwrap().border, UiTheme::default().accent);
    /// ```
    pub fn border_width(self, width: f32) -> Self {
        self.paint_with(|p| p.border_width = width)
    }
    /// Font handle, from `Graphics::load_font`.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let id = ui.label(root, "Hi").font(Font::DEFAULT).id();
    /// ```
    pub fn font(self, font: Font) -> Self {
        self.paint_with(|p| p.font = font)
    }
    /// Break text at word boundaries.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let id = ui.label(root, "A long story").wrap().id();
    /// assert!(ui.paint(id).unwrap().wrap);
    /// ```
    pub fn wrap(self) -> Self {
        self.paint_with(|p| p.wrap = true)
    }
    /// Horizontal text alignment.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let id = ui.label(root, "Hi").align(UiTextAlign::Center).id();
    /// assert_eq!(ui.paint(id).unwrap().align, UiTextAlign::Center);
    /// ```
    pub fn align(self, align: UiTextAlign) -> Self {
        self.paint_with(|p| p.align = align)
    }
    /// Image or background texture; `slice > 0` draws it as nine patches
    /// with that source border in pixels.
    /// ```
    /// use ponko::*;
    /// # fn build(ui: &mut Ui, root: UiId, frame: Texture) {
    /// let id = ui.panel(root).texture(frame, 4.0).id();
    /// assert_eq!(ui.paint(id).unwrap().slice, 4.0);
    /// # }
    /// ```
    pub fn texture(self, texture: Texture, slice: f32) -> Self {
        self.paint_with(|p| {
            p.texture = Some(texture);
            p.slice = slice;
        })
    }
    /// Replace the whole paint; see [`Ui::set_paint`].
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().paint(UiPaint { wrap: true, ..Default::default() }).id();
    /// assert!(ui.paint(root).unwrap().wrap);
    /// ```
    pub fn paint(self, paint: UiPaint) -> Self {
        self.ui.set_paint(self.id, paint);
        self
    }
    /// Enable or disable interaction; see [`Ui::set_enabled`].
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let buy = ui.button(root, "Buy").enabled(false).id();
    /// ```
    pub fn enabled(self, enabled: bool) -> Self {
        self.ui.set_enabled(self.id, enabled);
        self
    }
    /// Slider (0..1) or toggle (0 off, 1 on) value; see [`Ui::set_value`].
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let volume = ui.slider(root).value(0.8).id();
    /// assert_eq!(ui.value(volume), Some(0.8));
    /// ```
    pub fn value(self, value: f32) -> Self {
        self.ui.set_value(self.id, value);
        self
    }
    /// Name read by assistive technology; see [`Ui::set_accessible_name`].
    /// Give one to sliders, text inputs, images and icon-only buttons.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let volume = ui.slider(root).accessible_name("Volume").id();
    /// assert_eq!(ui.accessible_name(volume), Some("Volume"));
    /// ```
    pub fn accessible_name(self, name: impl AsRef<str>) -> Self {
        self.ui.set_accessible_name(self.id, name);
        self
    }
}

impl Ui {
    fn build(&mut self, parent: UiId, widget: UiWidget) -> UiBuilder<'_> {
        let id = self.add_child(parent, widget, Style::default());
        UiBuilder { ui: self, id }
    }
    /// A detached panel, to fill with children and push as a screen.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fill()).id();
    /// ui.push_screen(root, false);
    /// ```
    pub fn root(&mut self) -> UiBuilder<'_> {
        let id = self.add(UiWidget::Panel, Style::default());
        UiBuilder { ui: self, id }
    }
    /// A child container. Panics, like [`Ui::add_child`], if `parent` was removed.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let bar = ui.panel(root).layout(UiLayout::row(40.0)).id();
    /// ```
    pub fn panel(&mut self, parent: UiId) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Panel)
    }
    /// A text label.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let score = ui.label(root, "Score: 0").id();
    /// ```
    pub fn label(&mut self, parent: UiId, text: impl AsRef<str>) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Label).text(text)
    }
    /// A button; activating it queues [`crate::UiEvent::Clicked`].
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let play = ui.button(root, "Play").layout(UiLayout::fixed(200.0, 40.0)).id();
    /// ```
    pub fn button(&mut self, parent: UiId, text: impl AsRef<str>) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Button).text(text)
    }
    /// An image, stretched over the node.
    /// ```
    /// use ponko::*;
    /// # fn build(ui: &mut Ui, root: UiId, coin: Texture) {
    /// let icon = ui.image(root, coin).layout(UiLayout::fixed(32.0, 32.0)).id();
    /// # }
    /// ```
    pub fn image(&mut self, parent: UiId, texture: Texture) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Image).texture(texture, 0.0)
    }
    /// A clipped, scrolling container.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().layout(UiLayout::fill()).id();
    /// let list = ui.scroll_area(root).layout(UiLayout::fill()).id();
    /// ```
    pub fn scroll_area(&mut self, parent: UiId) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Scroll)
    }
    /// A single-line text field.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let chat = ui.text_input(root).layout(UiLayout::row(30.0)).id();
    /// ```
    pub fn text_input(&mut self, parent: UiId) -> UiBuilder<'_> {
        self.build(parent, UiWidget::TextInput)
    }
    /// A checkbox with a caption.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let music = ui.toggle(root, "Music").value(1.0).id();
    /// ```
    pub fn toggle(&mut self, parent: UiId, text: impl AsRef<str>) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Toggle).text(text)
    }
    /// A 0..1 slider.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let volume = ui.slider(root).layout(UiLayout::fixed(200.0, 24.0)).id();
    /// ```
    pub fn slider(&mut self, parent: UiId) -> UiBuilder<'_> {
        self.build(parent, UiWidget::Slider)
    }

    /// The current theme.
    /// ```
    /// use ponko::*;
    /// assert_eq!(*Ui::default().theme(), UiTheme::default());
    /// ```
    pub fn theme(&self) -> &UiTheme {
        &self.theme
    }
    /// Replace the theme. New nodes take their paint from it; existing nodes
    /// follow it in every paint field still equal to the old theme's value,
    /// and keep the fields set on them.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// let plain = ui.label(root, "a").id();
    /// let red = ui.label(root, "b").color(Color::RED).id();
    /// ui.set_theme(UiTheme { text: Color::YELLOW, ..Default::default() });
    /// assert_eq!(ui.paint(plain).unwrap().color, Color::YELLOW);
    /// assert_eq!(ui.paint(red).unwrap().color, Color::RED);
    /// ```
    pub fn set_theme(&mut self, theme: UiTheme) {
        let old = std::mem::replace(&mut self.theme, theme);
        for n in self.nodes.iter_mut().flatten() {
            let (from, to) = (old.paint(n.widget), theme.paint(n.widget));
            let p = &mut n.paint;
            macro_rules! follow {
                ($($field:ident),*) => {$(
                    if p.$field == from.$field {
                        p.$field = to.$field;
                    }
                )*};
            }
            follow!(
                background,
                hover,
                pressed,
                disabled,
                border,
                border_width,
                color,
                font,
                font_size,
                wrap,
                align,
                texture,
                slice
            );
        }
    }
    /// A node's resolved paint (theme plus its own settings), or `None` for a
    /// stale id.
    /// ```
    /// use ponko::*;
    /// let mut ui = Ui::default();
    /// let root = ui.root().id();
    /// assert_eq!(ui.paint(root).unwrap().background, UiTheme::default().panel);
    /// ```
    pub fn paint(&self, id: UiId) -> Option<UiPaint> {
        Some(self.node(id)?.paint)
    }
}
