//! The player's choices: UI scale, volume and rebindable controls, kept in a
//! `settings.json` next to the progress file, and the settings screen that
//! edits them.
//!
//! The screen is the engine's retained UI (`ctx.ui`), so it is the
//! reference for three things the engine already offers:
//!
//! - **Navigation.** Tab and Shift-Tab, the arrow keys or the d-pad move the
//!   focus, Enter or the gamepad's South button press the focused control,
//!   Escape or East closes the screen, and the list of controls scrolls to keep
//!   the focus in view. The game writes no navigation code.
//! - **UI scale.** `Ui::set_scale` scales the settings screen's geometry, text
//!   and hit testing together; `hud` applies the same factor to the text it
//!   draws itself.
//! - **Rebinding.** The installed `ActionMap` (`ctx.input.actions()`) is the
//!   one record of what is bound: the screen shows the keys it reads back
//!   from it, and a new key replaces one action in place with
//!   `ctx.input.set_action`, keeping the `ActionId`s the game resolved at
//!   startup. [`Controls`] is only the shape of the saved file:
//!   [`bindings_for`] turns it into a map when the game starts and
//!   [`Controls::from_bindings`] reads it back from the map before saving.
//!   Pressing a control's button opens a small prompt; the next key or button
//!   pressed becomes the new binding, unless it is reserved or already used.
//!
//! Limitations are listed in the book's chapter on this example.

use super::*;
use std::collections::HashSet;

/// Layout version of the settings file; another version is recovered from.
pub const SETTINGS_VERSION: u32 = 1;

/// The UI scales the settings offer. More than 1.5 would push the title
/// screen's text out of the window, less than 0.75 is too small to read.
pub const SCALES: [f32; 4] = [0.75, 1.0, 1.25, 1.5];

/// Something the player can do, for which a key (and a pad button) can be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Control {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Confirm,
    Restart,
    Pause,
    Back,
    Settings,
}

impl Control {
    pub const ALL: [Control; 9] = [
        Control::MoveUp,
        Control::MoveDown,
        Control::MoveLeft,
        Control::MoveRight,
        Control::Confirm,
        Control::Restart,
        Control::Pause,
        Control::Back,
        Control::Settings,
    ];

    /// The name on the settings screen.
    pub fn label(self) -> &'static str {
        match self {
            Control::MoveUp => "Move up",
            Control::MoveDown => "Move down",
            Control::MoveLeft => "Move left",
            Control::MoveRight => "Move right",
            Control::Confirm => "Confirm",
            Control::Restart => "Restart round",
            Control::Pause => "Pause / quit",
            Control::Back => "Back to title",
            Control::Settings => "Settings",
        }
    }

    /// The action this control's key and button feed: every movement key
    /// is one source of `"move"`, so movement has none of its own.
    pub fn action(self) -> Option<&'static str> {
        match self {
            Control::MoveUp | Control::MoveDown | Control::MoveLeft | Control::MoveRight => None,
            Control::Confirm => Some("confirm"),
            Control::Restart => Some("restart"),
            Control::Pause => Some("pause"),
            Control::Back => Some("back"),
            Control::Settings => Some("settings"),
        }
    }

    /// Movement is the keys here and the left stick on the pad; the stick is not rebindable.
    pub fn is_movement(self) -> bool {
        matches!(
            self,
            Control::MoveUp | Control::MoveDown | Control::MoveLeft | Control::MoveRight
        )
    }
}

/// Keys that always work next to the chosen ones (arrows for movement, Space
/// to confirm, P to pause). They can't be given to another control.
pub const FIXED_KEYS: [Key; 6] = [Key::Left, Key::Right, Key::Up, Key::Down, Key::Space, Key::P];

/// Why a key or button was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The retained UI owns them: Escape closes dialogs, Tab moves the focus,
    /// and on the gamepad East closes dialogs while the four D-pad directions
    /// move the focus. (Escape and East stay the defaults of Pause and Back;
    /// "Reset to defaults" gives them back.)
    Reserved,
    /// One of [`FIXED_KEYS`].
    Fixed,
    /// Another control has it.
    InUse(Control),
    /// Movement has no gamepad button (the stick moves).
    NoButton,
}

/// The gamepad buttons the retained UI uses for navigation, which no control
/// may take: the keyboard side reserves Escape, Tab and the arrow keys the same way.
pub fn is_reserved_pad(button: ponko::input::GamepadButton) -> bool {
    matches!(
        button,
        ponko::input::GamepadButton::East
            | ponko::input::GamepadButton::DPadUp
            | ponko::input::GamepadButton::DPadDown
            | ponko::input::GamepadButton::DPadLeft
            | ponko::input::GamepadButton::DPadRight
    )
}

/// The key, and the gamepad button, chosen for each [`Control`], in the order
/// of [`Control::ALL`]: what `settings.json` stores. The installed bindings are
/// the live record; [`Controls::from_bindings`] reads this back from them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Controls {
    pub keys: [Key; 9],
    pub pads: [Option<ponko::input::GamepadButton>; 9],
}

impl Default for Controls {
    fn default() -> Self {
        Controls {
            keys: [
                Key::W,
                Key::S,
                Key::A,
                Key::D,
                Key::Enter,
                Key::R,
                Key::Escape,
                Key::Q,
                Key::O,
            ],
            pads: [
                None,
                None,
                None,
                None,
                Some(ponko::input::GamepadButton::South),
                Some(ponko::input::GamepadButton::West),
                Some(ponko::input::GamepadButton::Start),
                Some(ponko::input::GamepadButton::East),
                Some(ponko::input::GamepadButton::North),
            ],
        }
    }
}

impl Controls {
    pub fn key(&self, control: Control) -> Key {
        self.keys[control as usize]
    }

    pub fn pad(&self, control: Control) -> Option<ponko::input::GamepadButton> {
        self.pads[control as usize]
    }

    /// The key as the player reads it: `Enter`, `Esc`, `1`.
    pub fn key_name(&self, control: Control) -> String {
        key_name(self.key(control))
    }

    // ANCHOR: from_bindings
    /// Reads the chosen keys and buttons back from bindings made by
    /// [`bindings_for`] (the installed ones are `ctx.input.actions()`). A
    /// control the map doesn't bind keeps its default.
    pub fn from_bindings(map: &ActionMap) -> Controls {
        let mut controls = Controls::default();
        if let Some(Axis2Binding::Keys { left, right, up, down }) = map.get("move").and_then(|a| a.axes2().first()) {
            controls.keys[Control::MoveLeft as usize] = *left;
            controls.keys[Control::MoveRight as usize] = *right;
            controls.keys[Control::MoveUp as usize] = *up;
            controls.keys[Control::MoveDown as usize] = *down;
        }
        for control in Control::ALL {
            let Some(action) = control.action().and_then(|name| map.get(name)) else {
                continue;
            };
            // The chosen key comes first, then the chosen button (if any),
            // then the fixed alternate key.
            let sources = action.buttons();
            if let Some(ButtonBinding::Key(key)) = sources.first() {
                controls.keys[control as usize] = *key;
            }
            controls.pads[control as usize] = sources.iter().find_map(|b| match b {
                ButtonBinding::Gamepad { button, .. } => Some(*button),
                _ => None,
            });
        }
        controls
    }
    // ANCHOR_END: from_bindings

    // ANCHOR: set_key
    /// Gives `control` the key, unless the key is not allowed. Nothing changes on refusal.
    pub fn set_key(&mut self, control: Control, key: Key) -> Result<(), Refused> {
        if matches!(key, Key::Escape | Key::Tab) {
            return Err(Refused::Reserved);
        }
        if FIXED_KEYS.contains(&key) {
            return Err(Refused::Fixed);
        }
        if let Some(other) = Control::ALL.into_iter().find(|&c| c != control && self.key(c) == key) {
            return Err(Refused::InUse(other));
        }
        self.keys[control as usize] = key;
        Ok(())
    }
    // ANCHOR_END: set_key

    /// Gives `control` the gamepad button, unless it is not allowed.
    pub fn set_pad(&mut self, control: Control, button: ponko::input::GamepadButton) -> Result<(), Refused> {
        if control.is_movement() {
            return Err(Refused::NoButton);
        }
        if is_reserved_pad(button) {
            return Err(Refused::Reserved);
        }
        if let Some(other) = Control::ALL
            .into_iter()
            .find(|&c| c != control && self.pad(c) == Some(button))
        {
            return Err(Refused::InUse(other));
        }
        self.pads[control as usize] = Some(button);
        Ok(())
    }

    /// Whether a settings file could have been written by this game: no key
    /// or button twice, no reserved or fixed key, no button on a movement.
    pub fn is_valid(&self) -> bool {
        let mut keys = HashSet::new();
        let mut pads = HashSet::new();
        Control::ALL.into_iter().all(|c| {
            let key = self.key(c);
            let key_ok = !FIXED_KEYS.contains(&key)
                && key != Key::Tab
                && (key != Key::Escape || c == Control::Pause)
                && keys.insert(key);
            let pad_ok = match self.pad(c) {
                None => true,
                // East is Back's default, so it is allowed there and nowhere else.
                Some(button) => {
                    !c.is_movement()
                        && (!is_reserved_pad(button)
                            || (button == ponko::input::GamepadButton::East && c == Control::Back))
                        && pads.insert(button)
                }
            };
            key_ok && pad_ok
        })
    }
}

/// A key as the player reads it: `Enter`, `Esc`, `1`.
pub fn key_name(key: Key) -> String {
    match key {
        Key::Escape => "Esc".to_string(),
        key => {
            let name = format!("{key:?}");
            match name.strip_prefix("Num") {
                Some(digit) if digit.len() == 1 => digit.to_string(),
                _ => name,
            }
        }
    }
}

// ANCHOR: bindings_for
/// The input bindings for `controls`: the chosen keys and buttons, the fixed
/// alternates, and the left stick.
pub fn bindings_for(controls: &Controls) -> ActionMap {
    let pad = ponko::input::Gamepad::from_slot(0).expect("slot 0 exists");
    let key = |c| controls.key(c);
    let mut map = ActionMap::default();
    map.set(
        "move",
        Action::Axis2(vec![
            Axis2Binding::Keys {
                left: key(Control::MoveLeft),
                right: key(Control::MoveRight),
                up: key(Control::MoveUp),
                down: key(Control::MoveDown),
            },
            Axis2Binding::Keys {
                left: Key::Left,
                right: Key::Right,
                up: Key::Up,
                down: Key::Down,
            },
            Axis2Binding::Gamepad {
                pad,
                stick: ponko::input::GamepadStick::Left,
                deadzone: 0.2,
            },
        ]),
    );
    let button = |control: Control, extra: Option<Key>| {
        let mut list = vec![ButtonBinding::Key(key(control))];
        if let Some(button) = controls.pad(control) {
            list.push(ButtonBinding::Gamepad { pad, button });
        }
        list.extend(extra.map(ButtonBinding::Key));
        Action::Button(list)
    };
    let fixed = |control| match control {
        Control::Confirm => Some(Key::Space),
        Control::Pause => Some(Key::P),
        _ => None,
    };
    for control in Control::ALL {
        if let Some(name) = control.action() {
            map.set(name, button(control, fixed(control)));
        }
    }
    map
}
// ANCHOR_END: bindings_for

/// What the player chose. A resource; the saved copy is `settings.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// One of [`SCALES`].
    pub ui_scale: f32,
    /// Sound effects, `0.0..=1.0`.
    pub volume: f32,
    pub controls: Controls,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            ui_scale: 1.0,
            volume: 0.8,
            controls: Controls::default(),
        }
    }
}

impl Settings {
    /// Reads `path`: like the progress file, a missing file is a first run and
    /// a damaged or impossible one is replaced by the defaults (and kept as `.bad`).
    pub fn load(path: &Path) -> (Settings, LoadStatus) {
        load_json(path, Settings::is_valid)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        save_json(path, self)
    }

    fn is_valid(&self) -> bool {
        self.version == SETTINGS_VERSION
            && SCALES.contains(&self.ui_scale)
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
            && self.controls.is_valid()
    }
}

/// The settings file lives beside the progress file: `None` when progress isn't kept.
pub fn settings_path(save: &Path) -> Option<PathBuf> {
    Some(save.parent()?.join("settings.json"))
}

/// Where the settings are kept and what happened to them. A resource.
#[derive(Debug, Clone)]
pub struct SettingsFile {
    pub path: Option<PathBuf>,
    pub load: LoadStatus,
    pub save_failed: bool,
}

/// Writes the settings and returns to the title screen. Called by `rules`.
pub fn leave_settings(ctx: &mut Ctx) {
    let settings = ctx.world.resource::<Settings>().clone();
    let file = ctx.world.resource_mut::<SettingsFile>();
    file.save_failed = match &file.path {
        Some(path) => settings.save(path).is_err(),
        None => false,
    };
    ctx.world.insert_resource(Screen::Title);
}

// ---- The settings screen ----------------------------------------------------

/// Keys the rebinding prompt listens for. `Key` can't be listed by the engine
/// (it only answers "is this key pressed"), so the game names the ones it offers.
fn candidate_keys() -> Vec<Key> {
    use Key::*;
    let mut keys = vec![
        A,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        M,
        N,
        O,
        P,
        Q,
        R,
        S,
        T,
        U,
        V,
        W,
        X,
        Y,
        Z,
        Num0,
        Num1,
        Num2,
        Num3,
        Num4,
        Num5,
        Num6,
        Num7,
        Num8,
        Num9,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        Up,
        Down,
        Left,
        Right,
        Space,
        Enter,
        Escape,
        Tab,
        Backspace,
        Insert,
        Delete,
        Home,
        End,
        PageUp,
        PageDown,
        LeftShift,
        RightShift,
        LeftCtrl,
        RightCtrl,
        LeftAlt,
        RightAlt,
        Minus,
        Equal,
        LeftBracket,
        RightBracket,
        Backslash,
        Semicolon,
        Apostrophe,
        Backquote,
        Comma,
        Period,
        Slash,
        Numpad0,
        Numpad1,
        Numpad2,
        Numpad3,
        Numpad4,
        Numpad5,
        Numpad6,
        Numpad7,
        Numpad8,
        Numpad9,
        NumpadAdd,
        NumpadSubtract,
        NumpadMultiply,
        NumpadDivide,
        NumpadDecimal,
        NumpadEnter,
    ];
    keys.shrink_to_fit();
    keys
}

/// Every gamepad button the prompt listens for (all sixteen), so it can say
/// "reserved" for the ones it refuses instead of ignoring them.
pub const PAD_BUTTONS: [ponko::input::GamepadButton; 16] = [
    ponko::input::GamepadButton::South,
    ponko::input::GamepadButton::East,
    ponko::input::GamepadButton::North,
    ponko::input::GamepadButton::West,
    ponko::input::GamepadButton::LeftBumper,
    ponko::input::GamepadButton::RightBumper,
    ponko::input::GamepadButton::LeftTrigger,
    ponko::input::GamepadButton::RightTrigger,
    ponko::input::GamepadButton::Select,
    ponko::input::GamepadButton::Start,
    ponko::input::GamepadButton::LeftStick,
    ponko::input::GamepadButton::RightStick,
    ponko::input::GamepadButton::DPadUp,
    ponko::input::GamepadButton::DPadDown,
    ponko::input::GamepadButton::DPadLeft,
    ponko::input::GamepadButton::DPadRight,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Device {
    Key,
    Pad,
}

/// A rebinding in progress: the prompt is open and the next press is the answer.
#[derive(Debug, Clone, Copy)]
struct Capture {
    control: Control,
    device: Device,
    /// False in the frame the prompt opened: the press that opened it (Enter on
    /// the focused button) is still "pressed this frame" and must not answer it.
    armed: bool,
}

struct Row {
    control: Control,
    key: UiId,
    pad: Option<UiId>,
}

/// The settings screen's widgets. A resource, built the first time the screen opens.
pub struct SettingsUi {
    root: UiId,
    prompt: UiId,
    prompt_text: UiId,
    status: UiId,
    scale_label: UiId,
    scale_down: UiId,
    scale_up: UiId,
    volume: UiId,
    reset: UiId,
    back: UiId,
    rows: Vec<Row>,
    /// The screen is pushed on the UI's stack.
    open: bool,
    capture: Option<Capture>,
}

impl SettingsUi {
    fn build(ui: &mut Ui, settings: &Settings) -> SettingsUi {
        let root = ui
            .root()
            .layout(ponko::ui::UiLayout::fill().padding(16.0).gap(6.0))
            .background(Color::hex(0x14181f))
            .id();
        ui.label(root, "Settings")
            .layout(ponko::ui::UiLayout::row(40.0))
            .size(28.0);

        let strip = ui.panel(root).layout(ponko::ui::UiLayout::row(34.0).gap(8.0)).id();
        ui.label(strip, "UI scale")
            .layout(ponko::ui::UiLayout::fixed(150.0, 34.0));
        let scale_down = ui
            .button(strip, "-")
            .layout(ponko::ui::UiLayout::fixed(40.0, 34.0))
            .accessible_name("Smaller UI")
            .id();
        let scale_label = ui.label(strip, "").layout(ponko::ui::UiLayout::fixed(70.0, 34.0)).id();
        let scale_up = ui
            .button(strip, "+")
            .layout(ponko::ui::UiLayout::fixed(40.0, 34.0))
            .accessible_name("Larger UI")
            .id();

        let strip = ui.panel(root).layout(ponko::ui::UiLayout::row(34.0).gap(8.0)).id();
        ui.label(strip, "Volume")
            .layout(ponko::ui::UiLayout::fixed(150.0, 34.0));
        let volume = ui
            .slider(strip)
            .layout(ponko::ui::UiLayout::fixed(240.0, 34.0))
            .value(settings.volume)
            .accessible_name("Volume")
            .id();

        ui.label(root, HINT)
            .layout(ponko::ui::UiLayout::row(52.0))
            .size(16.0)
            .wrap();
        let scroll = ui.scroll_area(root).layout(ponko::ui::UiLayout::fill()).id();
        let list = ui
            .panel(scroll)
            .layout(ponko::ui::UiLayout::auto().vertical().gap(4.0).width_fraction(0.95))
            .id();
        let mut rows = Vec::new();
        for control in Control::ALL {
            let strip = ui.panel(list).layout(ponko::ui::UiLayout::row(32.0).gap(8.0)).id();
            ui.label(strip, control.label())
                .layout(ponko::ui::UiLayout::fixed(150.0, 32.0));
            let key = ui
                .button(strip, "")
                .layout(ponko::ui::UiLayout::fixed(120.0, 32.0))
                .id();
            let pad = if control.is_movement() {
                ui.label(strip, "left stick")
                    .layout(ponko::ui::UiLayout::fixed(120.0, 32.0));
                None
            } else {
                Some(
                    ui.button(strip, "")
                        .layout(ponko::ui::UiLayout::fixed(120.0, 32.0))
                        .id(),
                )
            };
            rows.push(Row { control, key, pad });
        }

        let strip = ui.panel(root).layout(ponko::ui::UiLayout::row(36.0).gap(8.0)).id();
        let reset = ui
            .button(strip, "Reset to defaults")
            .layout(ponko::ui::UiLayout::fixed(190.0, 36.0))
            .id();
        let back = ui
            .button(strip, "Back")
            .layout(ponko::ui::UiLayout::fixed(120.0, 36.0))
            .id();
        let status = ui
            .label(root, "")
            .layout(ponko::ui::UiLayout::row(24.0))
            .size(16.0)
            .id();

        // The prompt: a modal screen with nothing to focus, so no key can press
        // a button behind it; Escape and East pop it (the engine does that).
        let prompt = ui
            .root()
            .layout(ponko::ui::UiLayout::centered(460.0, 110.0).padding(16.0).gap(8.0))
            .background(Color::hex(0x2b3240))
            .id();
        let prompt_text = ui
            .label(prompt, "")
            .layout(ponko::ui::UiLayout::fixed(420.0, 78.0))
            .wrap()
            .id();

        SettingsUi {
            root,
            prompt,
            prompt_text,
            status,
            scale_label,
            scale_down,
            scale_up,
            volume,
            reset,
            back,
            rows,
            open: false,
            capture: None,
        }
    }

    /// Shows the current settings in the labels and buttons: the scale and
    /// volume from `settings`, the keys and buttons read back from the
    /// installed `bindings`.
    fn refresh(&self, ui: &mut Ui, settings: &Settings, bindings: &ActionMap) {
        ui.set_text(self.scale_label, format!("{:.0}%", settings.ui_scale * 100.0));
        ui.set_value(self.volume, settings.volume);
        let bound = Controls::from_bindings(bindings);
        // A button's name is its text unless told otherwise; "Enter" alone says nothing.
        for row in &self.rows {
            let key = bound.key_name(row.control);
            ui.set_text(row.key, &key);
            ui.set_accessible_name(row.key, format!("{} key: {key}", row.control.label()));
            if let (Some(id), Some(button)) = (row.pad, bound.pad(row.control)) {
                ui.set_text(id, format!("{button:?}"));
                ui.set_accessible_name(id, format!("{} gamepad button: {button:?}", row.control.label()));
            }
        }
    }
}

/// Above the list of controls: how to rebind, and what can't be chosen.
pub const HINT: &str = "Press a button, then the new key. Escape, Tab, East and the D-pad are reserved for the menus; Reset gives Pause its Escape back.";

/// The sentence a refused key or button gets.
pub fn refusal(refused: Refused, name: &str, controls: &Controls) -> String {
    match refused {
        Refused::Reserved => format!("{name} is reserved for the menus. Press another."),
        Refused::Fixed => format!("{name} always works for something else. Press another."),
        Refused::InUse(other) => {
            let _ = controls;
            format!("{name} is already used for \"{}\". Press another.", other.label())
        }
        Refused::NoButton => "Movement uses the left stick.".to_string(),
    }
}

/// The next step of a UI scale: `+1` larger, `-1` smaller, stopping at the ends.
pub fn step_scale(current: f32, direction: i32) -> f32 {
    let index = SCALES.iter().position(|&s| s == current).unwrap_or(1) as i32;
    SCALES[(index + direction).clamp(0, SCALES.len() as i32 - 1) as usize]
}

// ANCHOR: settings_ui
/// Frame system: the settings screen. Runs after the engine has processed this
/// frame's UI input, so the focus has moved and the events are waiting.
///
/// It opens the screen when the game is on `Screen::Settings`, turns the UI's
/// events into changes of the [`Settings`] (and `Intent::close_settings` for Back
/// or Escape), runs the rebinding prompt, and closes the screen again when the
/// game has left it.
pub fn settings_ui(ctx: &mut Ctx) {
    let on_screen = *ctx.world.resource::<Screen>() == Screen::Settings;
    if ctx.world.get_resource::<SettingsUi>().is_none() {
        if !on_screen {
            return;
        }
        let settings = ctx.world.resource::<Settings>().clone();
        let built = SettingsUi::build(&mut ctx.ui, &settings);
        ctx.world.insert_resource(built);
    }
    let mut state = ctx.world.remove_resource::<SettingsUi>().expect("built above");
    drive(ctx, &mut state, on_screen);
    ctx.world.insert_resource(state);
}
// ANCHOR_END: settings_ui

fn drive(ctx: &mut Ctx, state: &mut SettingsUi, on_screen: bool) {
    if !on_screen {
        if state.open {
            // Left through the Back button (or `Intent`): take the screens off the stack.
            while ctx.ui.top_screen().is_some() {
                ctx.ui.pop_screen();
            }
            state.open = false;
            state.capture = None;
        }
        return;
    }
    if !state.open {
        let settings = ctx.world.resource::<Settings>().clone();
        state.refresh(&mut ctx.ui, &settings, ctx.input.actions());
        ctx.ui.set_text(state.status, "");
        ctx.ui.push_screen(state.root, false);
        state.open = true;
    }

    // Drain first, then change the tree (the events borrow it).
    let events: Vec<ponko::ui::UiEvent> = ctx.ui.events().collect();
    for event in events {
        match event {
            ponko::ui::UiEvent::Clicked(id) => click(ctx, state, id),
            ponko::ui::UiEvent::Changed(id) if id == state.volume => {
                let volume = ctx.ui.value(id).unwrap_or(0.8).clamp(0.0, 1.0);
                ctx.world.resource_mut::<Settings>().volume = volume;
            }
            ponko::ui::UiEvent::ScreenPopped(id) if id == state.prompt => state.capture = None,
            // Escape (or East) on the settings screen: leave, as the Back button does.
            ponko::ui::UiEvent::ScreenPopped(id) if id == state.root => {
                state.open = false;
                ctx.world.resource_mut::<Intent>().close_settings = true;
            }
            _ => {}
        }
    }
    if let Some(capture) = state.capture {
        capture_press(ctx, state, capture);
    }
}

fn click(ctx: &mut Ctx, state: &mut SettingsUi, id: UiId) {
    if id == state.back {
        ctx.world.resource_mut::<Intent>().close_settings = true;
    } else if id == state.scale_down || id == state.scale_up {
        let settings = ctx.world.resource_mut::<Settings>();
        settings.ui_scale = step_scale(settings.ui_scale, if id == state.scale_up { 1 } else { -1 });
        let settings = settings.clone();
        ctx.ui.set_scale(settings.ui_scale);
        state.refresh(&mut ctx.ui, &settings, ctx.input.actions());
    } else if id == state.reset {
        let settings = Settings::default();
        apply(ctx, &settings);
        state.refresh(&mut ctx.ui, &settings, ctx.input.actions());
        ctx.world.insert_resource(settings);
        ctx.ui.set_text(state.status, "Defaults restored");
    } else if let Some((control, device)) = state.rows.iter().find_map(|row| {
        if row.key == id {
            Some((row.control, Device::Key))
        } else if row.pad == Some(id) {
            Some((row.control, Device::Pad))
        } else {
            None
        }
    }) {
        let what = match device {
            Device::Key => "key",
            Device::Pad => "gamepad button",
        };
        ctx.ui.set_text(
            state.prompt_text,
            format!("Press the new {what} for \"{}\".\nEscape cancels.", control.label()),
        );
        ctx.ui.push_screen(state.prompt, true);
        state.capture = Some(Capture {
            control,
            device,
            armed: false,
        });
    }
}

/// Installs the bindings and scale of `settings`.
fn apply(ctx: &mut Ctx, settings: &Settings) {
    ctx.input.set_actions(bindings_for(&settings.controls));
    ctx.ui.set_scale(settings.ui_scale);
}

/// What the player pressed while the prompt was open, if anything.
fn pressed(ctx: &Ctx, device: Device) -> Option<(String, Result<Key, ponko::input::GamepadButton>)> {
    match device {
        Device::Key => candidate_keys()
            .into_iter()
            .find(|&k| ctx.key_pressed(k))
            .map(|k| (format!("{k:?}"), Ok(k))),
        // Every slot, connected or not: an unplugged pad has nothing pressed.
        Device::Pad => (0..).map_while(ponko::input::Gamepad::from_slot).find_map(|pad| {
            PAD_BUTTONS
                .into_iter()
                .find(|&b| ctx.input.gamepad_pressed(pad, b))
                .map(|b| (format!("{b:?}"), Err(b)))
        }),
    }
}

fn capture_press(ctx: &mut Ctx, state: &mut SettingsUi, capture: Capture) {
    if !capture.armed {
        state.capture = Some(Capture { armed: true, ..capture });
        return;
    }
    let Some((name, press)) = pressed(ctx, capture.device) else {
        return;
    };
    // ANCHOR: rebind
    // What is bound now, read back from the installed map: no copy to drift.
    let mut controls = Controls::from_bindings(ctx.input.actions());
    let result = match press {
        Ok(key) => controls.set_key(capture.control, key),
        Err(button) => controls.set_pad(capture.control, button),
    };
    match result {
        Ok(()) => {
            // Replace only the action this control feeds; its id stays valid.
            let action_name = capture.control.action().unwrap_or("move");
            let action = bindings_for(&controls).get(action_name).cloned();
            ctx.input
                .set_action(action_name, action.expect("bindings_for sets every action"));
            // The saved copy is read back from the map, too.
            let mut settings = ctx.world.resource::<Settings>().clone();
            settings.controls = Controls::from_bindings(ctx.input.actions());
            // ANCHOR_END: rebind
            state.refresh(&mut ctx.ui, &settings, ctx.input.actions());
            let message = format!("\"{}\" is now {name}", capture.control.label());
            ctx.world.insert_resource(settings);
            ctx.ui.set_text(state.status, message);
            ctx.ui.pop_screen(); // the prompt
            state.capture = None;
        }
        // Stay in the prompt and say why: pressing Escape still cancels.
        Err(refused) => {
            let message = refusal(refused, &name, &controls);
            ctx.ui.set_text(state.prompt_text, message);
        }
    }
}
