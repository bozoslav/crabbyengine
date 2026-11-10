//! Accessibility semantics for the retained UI: a read-only snapshot of roles,
//! names, values, states and focus, built from the existing node hierarchy.
//!
//! This is the platform-neutral half of screen-reader support. A native
//! adapter (a later stage) turns [`UiAccessTree`] into platform nodes; tests
//! and tools read it directly, without a window.

use super::retained::Ui;
use super::{UiId, UiWidget};
use crate::Rect;
use std::collections::HashMap;
use std::fmt;
use taffy::prelude::Display;

/// What a node is to assistive technology.
/// ```
/// use ponko::prelude::*;
/// assert_eq!(ponko::ui::UiRole::of(ponko::ui::UiWidget::Toggle), ponko::ui::UiRole::CheckBox);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiRole {
    /// A container: a panel or a non-modal screen root.
    Group,
    /// The root of a modal screen.
    Dialog,
    /// Static text.
    Label,
    /// Activates on press or confirm.
    Button,
    /// A picture; give it a name unless it is decorative.
    Image,
    /// A scrolling container.
    ScrollView,
    /// Single-line editable text; its text is the value, not the name.
    TextInput,
    /// An on/off toggle.
    CheckBox,
    /// A value from 0 to 1.
    Slider,
}

impl UiRole {
    /// The role a widget kind has outside a modal screen root.
    /// ```
    /// use ponko::prelude::*;
    /// assert_eq!(ponko::ui::UiRole::of(ponko::ui::UiWidget::Scroll), ponko::ui::UiRole::ScrollView);
    /// ```
    pub const fn of(widget: UiWidget) -> Self {
        match widget {
            UiWidget::Panel => Self::Group,
            UiWidget::Label => Self::Label,
            UiWidget::Button => Self::Button,
            UiWidget::Image => Self::Image,
            UiWidget::Scroll => Self::ScrollView,
            UiWidget::TextInput => Self::TextInput,
            UiWidget::Toggle => Self::CheckBox,
            UiWidget::Slider => Self::Slider,
        }
    }
    /// Lower-case name used by the snapshot text.
    /// ```
    /// use ponko::prelude::*;
    /// assert_eq!(ponko::ui::UiRole::TextInput.as_str(), "text-input");
    /// ```
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Dialog => "dialog",
            Self::Label => "label",
            Self::Button => "button",
            Self::Image => "image",
            Self::ScrollView => "scroll-view",
            Self::TextInput => "text-input",
            Self::CheckBox => "checkbox",
            Self::Slider => "slider",
        }
    }
}

/// A node's current value, by role.
/// ```
/// use ponko::prelude::*;
/// let mut ui = ponko::ui::Ui::default();
/// let root = ui.root().id();
/// let music = ui.toggle(root, "Music").value(1.0).id();
/// ui.push_screen(root, false);
/// ui.layout(vec2(800.0, 600.0));
/// assert_eq!(ui.accessibility_tree().get(music).unwrap().value, ponko::ui::UiAccessValue::Checked(true));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum UiAccessValue {
    /// No value (groups, labels, buttons, images).
    None,
    /// A text input's text.
    Text(String),
    /// A toggle's state.
    Checked(bool),
    /// A slider's value, from 0 to 1; keyboard and gamepad steps are 0.05.
    Range(f32),
}

/// One exposed node of a [`UiAccessTree`].
/// ```
/// use ponko::prelude::*;
/// let mut ui = ponko::ui::Ui::default();
/// let root = ui.root().id();
/// let play = ui.button(root, "Play").id();
/// ui.push_screen(root, false);
/// ui.layout(vec2(800.0, 600.0));
/// let tree = ui.accessibility_tree();
/// let node = tree.get(play).unwrap();
/// assert_eq!((node.role, node.name.as_str(), node.parent), (ponko::ui::UiRole::Button, "Play", Some(root)));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UiAccessNode {
    /// The UI node.
    pub id: UiId,
    /// Exposed parent; `None` for a screen root.
    pub parent: Option<UiId>,
    /// Exposed children, in layout order.
    pub children: Vec<UiId>,
    /// Nesting below the screen root (which is 0).
    pub depth: usize,
    /// What the node is.
    pub role: UiRole,
    /// Accessible name: the override from [`Ui::set_accessible_name`], else
    /// the text (except for text inputs, whose text is the value).
    pub name: String,
    /// Current value.
    pub value: UiAccessValue,
    /// Bounds in logical window coordinates, including the user scale.
    pub rect: Rect,
    /// Keyboard/gamepad navigation can focus it right now.
    pub focusable: bool,
    /// It has keyboard/gamepad focus.
    pub focused: bool,
    /// It or an ancestor is disabled.
    pub disabled: bool,
    /// It is on a screen below the top one: readable, but not interactive.
    pub inert: bool,
    /// It is the root of a modal screen.
    pub modal: bool,
}

/// A deterministic snapshot of the UI's accessibility semantics.
///
/// Nodes are in depth-first layout order, screens from the bottom up. A modal
/// screen hides every screen below it; non-modal lower screens stay readable
/// but [`inert`](UiAccessNode::inert). Nodes with `display: none` (and their
/// subtrees) are left out. Take the snapshot after [`Ui::frame`] or
/// [`Ui::layout`] so bounds and focus are current. Its `Display` form is one
/// line per node, indented by depth, for golden tests and tools.
/// ```
/// use ponko::prelude::*;
/// let mut ui = ponko::ui::Ui::default();
/// let root = ui.root().layout(ponko::ui::UiLayout::fill()).id();
/// ui.button(root, "Play");
/// ui.slider(root).layout(ponko::ui::UiLayout::row(20.0)).value(0.25).accessible_name("Volume");
/// ui.push_screen(root, false);
/// ui.layout(vec2(800.0, 600.0));
/// assert_eq!(
///     ui.accessibility_tree().to_string(),
///     "group\n  button \"Play\" focusable\n  slider \"Volume\" value=0.25 focusable\n"
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiAccessTree {
    nodes: Vec<UiAccessNode>,
    index: HashMap<UiId, usize>,
    screens: Vec<UiId>,
    focus: Option<UiId>,
}

impl UiAccessTree {
    /// All exposed nodes, depth-first, screens from the bottom up.
    /// ```
    /// use ponko::prelude::*;
    /// assert!(ponko::ui::Ui::default().accessibility_tree().nodes().is_empty());
    /// ```
    pub fn nodes(&self) -> &[UiAccessNode] {
        &self.nodes
    }
    /// The exposed node for `id`, or `None` if it is hidden, removed or on a
    /// screen covered by a modal one.
    /// ```
    /// use ponko::prelude::*;
    /// assert!(ponko::ui::Ui::default().accessibility_tree().get(ponko::ui::UiId::from_bits(0)).is_none());
    /// ```
    pub fn get(&self, id: UiId) -> Option<&UiAccessNode> {
        self.index.get(&id).map(|&i| &self.nodes[i])
    }
    /// Exposed screen roots, from the bottom up.
    /// ```
    /// use ponko::prelude::*;
    /// assert!(ponko::ui::Ui::default().accessibility_tree().screens().is_empty());
    /// ```
    pub fn screens(&self) -> &[UiId] {
        &self.screens
    }
    /// The focused node, if it is exposed.
    /// ```
    /// use ponko::prelude::*;
    /// assert_eq!(ponko::ui::Ui::default().accessibility_tree().focus(), None);
    /// ```
    pub fn focus(&self) -> Option<UiId> {
        self.focus
    }
}

impl fmt::Display for UiAccessTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for n in &self.nodes {
            write!(f, "{:indent$}{}", "", n.role.as_str(), indent = n.depth * 2)?;
            if !n.name.is_empty() {
                write!(f, " {:?}", n.name)?;
            }
            match &n.value {
                UiAccessValue::None => {}
                UiAccessValue::Text(text) => write!(f, " value={text:?}")?,
                UiAccessValue::Checked(true) => f.write_str(" checked")?,
                UiAccessValue::Checked(false) => f.write_str(" unchecked")?,
                UiAccessValue::Range(v) => write!(f, " value={v:.2}")?,
            }
            for (on, flag) in [
                (n.modal, "modal"),
                (n.disabled, "disabled"),
                (n.inert, "inert"),
                (n.focusable, "focusable"),
                (n.focused, "focused"),
            ] {
                if on {
                    write!(f, " {flag}")?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Ui {
    /// Override the name assistive technology reads for a node. By default
    /// the name is the node's text (none for text inputs, whose text is their
    /// value); sliders, text inputs, images and icon-only buttons need one.
    /// An empty name restores the default. Names never change layout,
    /// drawing, focus or events.
    /// ```
    /// use ponko::prelude::*;
    /// let mut ui = ponko::ui::Ui::default();
    /// let root = ui.root().id();
    /// let close = ui.button(root, "X").id();
    /// ui.set_accessible_name(close, "Close");
    /// assert_eq!(ui.accessible_name(close), Some("Close"));
    /// assert_eq!(ui.text(close), Some("X"));
    /// ```
    pub fn set_accessible_name(&mut self, id: UiId, name: impl AsRef<str>) {
        if let Some(n) = self.node_mut(id) {
            n.access_name.clear();
            n.access_name.push_str(name.as_ref());
        }
    }
    /// The name assistive technology reads, or `None` for a stale id.
    /// ```
    /// use ponko::prelude::*;
    /// let mut ui = ponko::ui::Ui::default();
    /// let root = ui.root().id();
    /// let play = ui.button(root, "Play").id();
    /// assert_eq!(ui.accessible_name(play), Some("Play"));
    /// ```
    pub fn accessible_name(&self, id: UiId) -> Option<&str> {
        let n = self.node(id)?;
        Some(if !n.access_name.is_empty() {
            &n.access_name
        } else if n.widget == UiWidget::TextInput {
            ""
        } else {
            &n.text
        })
    }
    /// Snapshot the accessibility semantics; see [`UiAccessTree`]. This
    /// allocates, so take it when the UI changes or a tool asks, not as part
    /// of every frame's hot path.
    /// ```
    /// use ponko::prelude::*;
    /// let mut ui = ponko::ui::Ui::default();
    /// let root = ui.root().id();
    /// let play = ui.button(root, "Play").id();
    /// ui.push_screen(root, false);
    /// ui.frame(vec2(800.0, 600.0), &ponko::ui::UiInput { next: true, ..Default::default() });
    /// assert_eq!(ui.accessibility_tree().focus(), Some(play));
    /// ```
    pub fn accessibility_tree(&self) -> UiAccessTree {
        let mut tree = UiAccessTree::default();
        let first = self.screens.iter().rposition(|s| s.modal).unwrap_or(0);
        let top = self.top_screen();
        for screen in self.screens.iter().skip(first) {
            if self.add_access(
                &mut tree,
                screen.root,
                None,
                0,
                Some(screen.root) != top,
                false,
                screen.modal,
            ) {
                tree.screens.push(screen.root);
            }
        }
        tree.focus = self.focus().filter(|id| tree.index.contains_key(id));
        tree
    }
    /// Add `id` and its shown subtree; false if it is hidden.
    #[allow(clippy::too_many_arguments)]
    fn add_access(
        &self,
        tree: &mut UiAccessTree,
        id: UiId,
        parent: Option<UiId>,
        depth: usize,
        inert: bool,
        disabled: bool,
        modal: bool,
    ) -> bool {
        let Some(n) = self.node(id) else { return false };
        if self.tree.style(n.layout).is_ok_and(|s| s.display == Display::None) {
            return false;
        }
        let disabled = disabled || !n.enabled;
        let role = if modal { UiRole::Dialog } else { UiRole::of(n.widget) };
        let value = match n.widget {
            UiWidget::TextInput => UiAccessValue::Text(n.text.clone()),
            UiWidget::Toggle => UiAccessValue::Checked(n.value >= 0.5),
            UiWidget::Slider => UiAccessValue::Range(n.value),
            _ => UiAccessValue::None,
        };
        let at = tree.nodes.len();
        tree.index.insert(id, at);
        tree.nodes.push(UiAccessNode {
            id,
            parent,
            children: Vec::new(),
            depth,
            role,
            name: self.accessible_name(id).unwrap_or_default().to_owned(),
            value,
            rect: n.rect,
            focusable: self.focusable(id),
            focused: self.focus() == Some(id),
            disabled,
            inert,
            modal,
        });
        for &child in &n.children {
            if self.add_access(tree, child, Some(id), depth + 1, inert, disabled, false) {
                tree.nodes[at].children.push(child);
            }
        }
        true
    }
}
