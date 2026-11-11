//! The native accessibility adapter: gives the retained UI's semantics to
//! the system's accessibility API, so a screen reader can read and operate
//! `ctx.ui`, and brings the screen reader's requests back to the UI.
//!
//! One backend exists: macOS (VoiceOver and anything else that speaks
//! `NSAccessibility`), through AccessKit's macOS adapter. On every other
//! system the adapter is inert: it builds, never activates, and costs
//! nothing.

use super::access_kit;
use super::{Ui, UiAccessTracker};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the system's callbacks and the adapter share. Both run on the main
/// thread, so no lock is needed (and the adapter is not `Send`).
#[derive(Default)]
struct Shared {
    /// Assistive technology asked for the tree: there is someone to tell.
    activated: Cell<bool>,
    /// Requests that arrived since the last update.
    requests: RefCell<Vec<accesskit::ActionRequest>>,
}

/// Connects one [`Ui`] to the system's accessibility API (feature
/// `accessibility`).
///
/// The engine makes one for the game's window and `ctx.ui` and drives it
/// every frame; nothing needs to be called from game code. A custom host
/// that owns its own view makes one with [`for_ns_view`](Self::for_ns_view)
/// and calls [`update`](Self::update) once a frame, after [`Ui::frame`].
///
/// Until assistive technology asks for the tree the adapter does nothing:
/// no snapshot is taken and nothing is allocated. Once it has, the first
/// update sends the whole tree and later ones only the nodes that changed
/// (see [`UiAccessTracker`]). Requests (press, focus, set value, increment,
/// decrement) are queued for the UI's next frame as
/// [`UiAccessAction`](super::UiAccessAction)s.
///
/// Supported on macOS. Elsewhere [`SUPPORTED`](Self::SUPPORTED) is false and
/// every method is a no-op.
/// ```
/// let mut adapter = ponko::ui::UiAccessAdapter::inert();
/// let mut ui = ponko::ui::Ui::default();
/// adapter.update(&mut ui, 1.0);
/// assert!(!adapter.is_active());
/// ```
pub struct UiAccessAdapter {
    tracker: UiAccessTracker,
    shared: Rc<Shared>,
    title: String,
    /// The scale factor the platform side last heard.
    scale_factor: f64,
    /// The platform side has the whole tree; changes are enough from now on.
    sent: bool,
    #[cfg(target_os = "macos")]
    platform: Option<accesskit_macos::SubclassingAdapter>,
}

impl UiAccessAdapter {
    /// Whether this build has a native backend for the system it runs on.
    /// ```
    /// assert_eq!(ponko::ui::UiAccessAdapter::SUPPORTED, cfg!(target_os = "macos"));
    /// ```
    pub const SUPPORTED: bool = cfg!(target_os = "macos");

    /// An adapter connected to nothing: it never activates. What the engine
    /// uses where there is no backend.
    /// ```
    /// assert!(!ponko::ui::UiAccessAdapter::inert().is_active());
    /// ```
    pub fn inert() -> Self {
        Self {
            tracker: UiAccessTracker::new(),
            shared: Rc::default(),
            title: String::new(),
            scale_factor: 1.0,
            sent: false,
            #[cfg(target_os = "macos")]
            platform: None,
        }
    }

    /// An adapter for an `NSView`: the view answers the system's
    /// accessibility queries from then on. `title` names the window to the
    /// screen reader. Call it on the main thread, before the view is shown
    /// or focused for the first time.
    ///
    /// # Safety
    ///
    /// `view` must be a valid, unreleased pointer to an `NSView`, and no
    /// other adapter may be attached to it.
    ///
    /// # Panics
    ///
    /// Off the main thread, or if the view already has an adapter.
    #[cfg(target_os = "macos")]
    pub unsafe fn for_ns_view(view: *mut std::ffi::c_void, title: &str) -> Self {
        let mut adapter = Self::inert();
        adapter.title.push_str(title);
        // SAFETY: the caller's contract is the adapter's.
        adapter.platform = Some(unsafe {
            accesskit_macos::SubclassingAdapter::new(
                view,
                Activation(adapter.shared.clone()),
                Requests(adapter.shared.clone()),
            )
        });
        adapter
    }

    /// The adapter for the engine's window, or an inert one where there is
    /// no backend. The window must not have been shown yet.
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    pub(crate) fn for_window(window: &winit::window::Window, title: &str) -> Self {
        #[cfg(target_os = "macos")]
        {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(RawWindowHandle::AppKit(handle)) = window.window_handle().map(|h| h.as_raw()) {
                // SAFETY: winit hands out its own live view, on the main
                // thread (windows are only made there), and nothing else
                // attaches an adapter to it.
                return unsafe { Self::for_ns_view(handle.ns_view.as_ptr(), title) };
            }
        }
        Self::inert()
    }

    /// Assistive technology has asked for the tree, so updates are being
    /// sent. False until a screen reader (or an accessibility inspector)
    /// looks at the window.
    /// ```
    /// assert!(!ponko::ui::UiAccessAdapter::inert().is_active());
    /// ```
    pub fn is_active(&self) -> bool {
        self.shared.activated.get()
    }

    /// The name the screen reader gives the window. The engine sets it once,
    /// from the game's title: its window keeps the title it was created
    /// with (nothing changes it while the game runs). A host that renames
    /// its own window calls this again.
    /// ```
    /// let mut adapter = ponko::ui::UiAccessAdapter::inert();
    /// adapter.set_title("Settings");
    /// ```
    pub fn set_title(&mut self, title: &str) {
        if self.title != title {
            self.title.clear();
            self.title.push_str(title);
            // The window node carries it: send that node again.
            self.scale_factor = f64::NAN;
        }
    }

    /// Tell the system whether the window has keyboard focus; the focused
    /// node is only reported as focused while it does.
    /// ```
    /// ponko::ui::UiAccessAdapter::inert().set_window_focused(true);
    /// ```
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    pub fn set_window_focused(&mut self, focused: bool) {
        #[cfg(target_os = "macos")]
        if let Some(events) = self.platform.as_mut().and_then(|p| p.update_view_focus_state(focused)) {
            events.raise();
        }
    }

    /// Once a frame, after [`Ui::frame`]: queue the requests that arrived
    /// for the UI's next frame, and, if assistive technology is listening,
    /// send what changed. `scale_factor` is the window's (physical pixels
    /// per logical unit); the UI's bounds are logical.
    /// ```
    /// use ponko::prelude::*;
    /// let mut ui = ponko::ui::Ui::default();
    /// let root = ui.root().id();
    /// ui.push_screen(root, false);
    /// ui.frame(vec2(800.0, 600.0), &ponko::ui::UiInput::default());
    /// let mut adapter = ponko::ui::UiAccessAdapter::inert();
    /// adapter.update(&mut ui, 2.0);
    /// ```
    pub fn update(&mut self, ui: &mut Ui, scale_factor: f32) {
        for request in self.shared.requests.borrow_mut().drain(..) {
            if let Some(action) = access_kit::action(&request) {
                ui.push_access_action(action);
            }
        }
        if !self.is_active() {
            return;
        }
        let scale_factor = f64::from(scale_factor);
        let window_changed = self.scale_factor != scale_factor;
        let changes = self.tracker.update(ui);
        if changes.is_none() && !window_changed && self.sent {
            return;
        }
        let changes = changes.unwrap_or_default();
        self.scale_factor = scale_factor;
        let (tree, title, sent) = (self.tracker.tree(), self.title.as_str(), self.sent);
        // Made only if the platform side takes it.
        #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
        let update = || {
            if sent {
                access_kit::partial_update(tree, &changes, title, scale_factor, window_changed)
            } else {
                access_kit::full_update(tree, title, scale_factor)
            }
        };
        #[cfg(target_os = "macos")]
        if let Some(platform) = self.platform.as_mut() {
            if let Some(events) = platform.update_if_active(update) {
                events.raise();
            }
            self.sent = true;
        }
    }
}

impl std::fmt::Debug for UiAccessAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiAccessAdapter")
            .field("supported", &Self::SUPPORTED)
            .field("active", &self.is_active())
            .field("sent", &self.sent)
            .finish_non_exhaustive()
    }
}

/// The system asks for the tree the first time assistive technology looks.
/// There is no `Ui` to read here, so the answer is "not yet": the next
/// [`UiAccessAdapter::update`] sends the whole tree.
#[cfg(target_os = "macos")]
struct Activation(Rc<Shared>);

#[cfg(target_os = "macos")]
impl accesskit::ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        self.0.activated.set(true);
        None
    }
}

/// Requests are queued; the next update hands them to the UI.
#[cfg(target_os = "macos")]
struct Requests(Rc<Shared>);

#[cfg(target_os = "macos")]
impl accesskit::ActionHandler for Requests {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        self.0.requests.borrow_mut().push(request);
    }
}
