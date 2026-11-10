//! Shapes, UI widgets, fonts, sound, screenshots and turns: the rest of `Ctx`.
use std::ffi::c_char;

use ponko::ui::UiStyle;
use ponko::{Font, Playing, Sound, SoundParams, TextParams};

use crate::ctx::{ctx_access, out, text};
use crate::types::*;
use crate::{CrabbyCtx, CrabbyStatus, boundary, invalid};

/// A font from `crabby_ctx_load_font`; `{0}` is the built-in font.
/// ```
/// assert_eq!(ponko_ffi::CrabbyFont::default().id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyFont {
    /// Engine-internal id; 0 is the built-in font.
    pub id: u32,
}

/// Options for `crabby_ctx_draw_text_ex`. Start from `crabby_text_params_default()`.
/// ```
/// assert_eq!(ponko_ffi::crabby_text_params_default().size, 16.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTextParams {
    /// Which font. Default the built-in one.
    pub font: CrabbyFont,
    /// Height of a line in window pixels. Default 16.
    pub size: f32,
    /// Default white.
    pub color: CrabbyColor,
    /// Draw order among screen-space drawing; higher is on top. Default 0.
    pub z: f32,
}

/// The default text options: built-in font, 16 px, white.
/// ```
/// assert_eq!(ponko_ffi::crabby_text_params_default().font.id, 0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_text_params_default() -> CrabbyTextParams {
    let p = TextParams::default();
    CrabbyTextParams {
        font: font_to_c(p.font),
        size: p.size,
        color: to_c(p.color),
        z: p.z,
    }
}

/// Colors and sizes of the UI widgets (`crabby_ctx_ui_*`).
/// ```
/// assert_eq!(ponko_ffi::crabby_ui_style_default().text_size, 16.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyUiStyle {
    /// Panel background.
    pub panel: CrabbyColor,
    /// Button background.
    pub button: CrabbyColor,
    /// Button under the mouse.
    pub button_hovered: CrabbyColor,
    /// Button held down.
    pub button_pressed: CrabbyColor,
    /// Highlights, such as a hovered button's outline.
    pub accent: CrabbyColor,
    /// Label and button text.
    pub text: CrabbyColor,
    /// Text height in pixels.
    pub text_size: f32,
    /// Space between a panel's edge and its contents, in pixels.
    pub padding: f32,
}

fn style_to_c(s: UiStyle) -> CrabbyUiStyle {
    CrabbyUiStyle {
        panel: to_c(s.panel),
        button: to_c(s.button),
        button_hovered: to_c(s.button_hovered),
        button_pressed: to_c(s.button_pressed),
        accent: to_c(s.accent),
        text: to_c(s.text),
        text_size: s.text_size,
        padding: s.padding,
    }
}

/// The engine's default UI style.
/// ```
/// assert_eq!(ponko_ffi::crabby_ui_style_default().padding, 10.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_style_default() -> CrabbyUiStyle {
    style_to_c(UiStyle::default())
}

/// A sound from `crabby_ctx_load_sound` or `crabby_ctx_create_sound`.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbySound>(), 8);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbySound {
    /// Engine-internal id. Never make one up.
    pub id: u32,
    /// Length in seconds.
    pub seconds: f32,
}

/// One playback of a sound, for `crabby_ctx_stop_sound`.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyPlaying>(), 8);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyPlaying {
    /// Engine-internal id; 0 in headless runs.
    pub id: u64,
}

/// Options for `crabby_ctx_play_sound`. Start from `crabby_sound_params_default()`.
/// ```
/// assert_eq!(ponko_ffi::crabby_sound_params_default().volume, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbySoundParams {
    /// 0 is silent, 1 (default) is as recorded.
    pub volume: f32,
    /// Playback speed, which also changes the pitch. Default 1.
    pub speed: f32,
    /// Start over at the end. Default false.
    pub looping: bool,
}

/// Default playback: full volume, normal speed, once.
/// ```
/// assert!(!ponko_ffi::crabby_sound_params_default().looping);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sound_params_default() -> CrabbySoundParams {
    let p = SoundParams::default();
    CrabbySoundParams {
        volume: p.volume,
        speed: p.speed,
        looping: p.looping,
    }
}

/// Which entities take their turn this tick, from `crabby_ctx_turns`:
/// entity `e` does when `e.index % every == now`. `dt` covers a whole round
/// (`every` ticks).
/// ```
/// let t = ponko_ffi::CrabbyTurns { every: 4, now: 1, dt: 0.0 };
/// assert_eq!(5 % t.every, t.now);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTurns {
    /// Ticks in a round.
    pub every: u32,
    /// This tick's place in the round.
    pub now: u32,
    /// Seconds a round covers.
    pub dt: f32,
}

// Font, Sound and Playing are repr(C)/repr(transparent) over these fields.
const _: () = {
    assert!(size_of::<CrabbyFont>() == size_of::<Font>() && align_of::<CrabbyFont>() == align_of::<Font>());
    assert!(size_of::<CrabbySound>() == size_of::<Sound>() && align_of::<CrabbySound>() == align_of::<Sound>());
    assert!(size_of::<CrabbyPlaying>() == size_of::<Playing>() && align_of::<CrabbyPlaying>() == align_of::<Playing>());
};
pub(crate) fn font_to_c(f: Font) -> CrabbyFont {
    // SAFETY: Font is repr(transparent) over u32 (size checked above).
    unsafe { std::mem::transmute(f) }
}
pub(crate) fn font_from_c(f: CrabbyFont) -> Font {
    // SAFETY: as above. Unknown ids make the call fail, they are not UB.
    unsafe { std::mem::transmute(f) }
}
pub(crate) fn sound_to_c(s: Sound) -> CrabbySound {
    // SAFETY: Sound is repr(C) { u32 id, f32 seconds }.
    unsafe { std::mem::transmute(s) }
}
pub(crate) fn sound_from_c(s: CrabbySound) -> Sound {
    // SAFETY: as above. Unknown ids make the call fail, they are not UB.
    unsafe { std::mem::transmute(s) }
}
pub(crate) fn playing_to_c(p: Playing) -> CrabbyPlaying {
    // SAFETY: Playing is repr(transparent) over u64.
    unsafe { std::mem::transmute(p) }
}
pub(crate) fn playing_from_c(p: CrabbyPlaying) -> Playing {
    // SAFETY: as above; stopping an unknown id does nothing.
    unsafe { std::mem::transmute(p) }
}

// ---- shapes ---------------------------------------------------------------------

/// Fills a rectangle in world space.
/// ```
/// use ponko_ffi::*;
/// let r = CrabbyRect::default();
/// let white = CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
/// assert_eq!(unsafe { crabby_ctx_draw_rect(std::ptr::null_mut(), r, white) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_rect(
    ctx: *mut CrabbyCtx,
    rect: CrabbyRect,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.draw_rect(to_c(rect), to_c(color));
            Ok(())
        })
    })
}

/// Draws a line `width` world units thick from `a` to `b`.
/// ```
/// use ponko_ffi::*;
/// let v = CrabbyVec2::default();
/// let white = CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
/// assert_eq!(unsafe { crabby_ctx_draw_line(std::ptr::null_mut(), v, v, 1.0, white) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_line(
    ctx: *mut CrabbyCtx,
    a: CrabbyVec2,
    b: CrabbyVec2,
    width: f32,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.draw_line(to_c(a), to_c(b), width, to_c(color));
            Ok(())
        })
    })
}

/// Fills a circle in world space.
/// ```
/// use ponko_ffi::*;
/// let white = CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
/// assert_eq!(unsafe { crabby_ctx_draw_circle(std::ptr::null_mut(), CrabbyVec2::default(), 4.0, white) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_circle(
    ctx: *mut CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.draw_circle(to_c(center), radius, to_c(color));
            Ok(())
        })
    })
}

// ---- UI ---------------------------------------------------------------------------

/// Reads the UI style.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_style(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `out_style` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_style(ctx: *mut CrabbyCtx, out_style: *mut CrabbyUiStyle) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_style, style_to_c(c.ui.immediate.style))) })
}

/// Changes the UI style for widgets drawn after this call.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_ui_style(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `style` must be readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_ui_style(ctx: *mut CrabbyCtx, style: *const CrabbyUiStyle) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if style.is_null() {
                return Err(invalid("style is null"));
            }
            let s = *style;
            c.ui.immediate.style = UiStyle {
                panel: to_c(s.panel),
                button: to_c(s.button),
                button_hovered: to_c(s.button_hovered),
                button_pressed: to_c(s.button_pressed),
                accent: to_c(s.accent),
                text: to_c(s.text),
                text_size: s.text_size,
                padding: s.padding,
            };
            Ok(())
        })
    })
}

/// Whether the mouse is over UI drawn this frame or the last, so clicks on
/// a panel don't also act on the world behind it.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_mouse_over_ui(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `out_over` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_over_ui(ctx: *mut CrabbyCtx, out_over: *mut bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_over, c.ui.wants_pointer())) })
}

/// Draws a panel background in screen space (window pixels).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_panel(std::ptr::null_mut(), Default::default()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_panel(ctx: *mut CrabbyCtx, rect: CrabbyRect) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.ui.immediate.panel(&mut c.gfx, to_c(rect));
            Ok(())
        })
    })
}

/// Draws text in the UI style at `pos` (screen space); writes its size to
/// `out_size` unless that is null.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_label(std::ptr::null_mut(), Default::default(), c"hi".as_ptr(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `string` NUL-terminated; `out_size` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_label(
    ctx: *mut CrabbyCtx,
    pos: CrabbyVec2,
    string: *const c_char,
    out_size: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let size = c.ui.immediate.label(&mut c.gfx, to_c(pos), text(string)?);
            if !out_size.is_null() {
                out_size.write(to_c(size));
            }
            Ok(())
        })
    })
}

/// Draws a button in screen space; `out_clicked` is true on the frame it was clicked.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_ui_button(std::ptr::null_mut(), Default::default(), c"ok".as_ptr(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `label` NUL-terminated; `out_clicked` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_button(
    ctx: *mut CrabbyCtx,
    rect: CrabbyRect,
    label: *const c_char,
    out_clicked: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let label = text(label)?;
            if out_clicked.is_null() {
                return Err(invalid("output is null"));
            }
            out(
                out_clicked,
                c.ui.immediate.button(&mut c.gfx, &c.input, to_c(rect), label),
            )
        })
    })
}

// ---- fonts --------------------------------------------------------------------------

/// Loads a TTF or OTF font from the assets folder.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_font(std::ptr::null_mut(), c"a.ttf".as_ptr(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `path` NUL-terminated; `out_font` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_font(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_font: *mut CrabbyFont,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_font.is_null() {
                return Err(invalid("output is null"));
            }
            out(out_font, font_to_c(c.gfx.load_font(path)))
        })
    })
}

/// Draws text in screen space with a font, size, color and z. An unknown or
/// freed font is an invalid argument.
/// ```
/// let p = ponko_ffi::crabby_text_params_default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_draw_text_ex(std::ptr::null_mut(), c"hi".as_ptr(), Default::default(), &p) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `string` NUL-terminated; `params` readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_text_ex(
    ctx: *mut CrabbyCtx,
    string: *const c_char,
    pos: CrabbyVec2,
    params: *const CrabbyTextParams,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let string = text(string)?;
            if params.is_null() {
                return Err(invalid("params is null"));
            }
            let p = *params;
            if !c.gfx.is_live(font_from_c(p.font)) {
                return Err(invalid("unknown or freed font"));
            }
            c.gfx.draw_text_ex(
                string,
                to_c(pos),
                TextParams {
                    font: font_from_c(p.font),
                    size: p.size,
                    color: to_c(p.color),
                    z: p.z,
                },
            );
            Ok(())
        })
    })
}

/// Size in pixels that `crabby_ctx_draw_text_ex` would cover with `font` and
/// `size`. An unknown or freed font is an invalid argument.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_measure_text_ex(std::ptr::null_mut(), c"hi".as_ptr(), Default::default(), 16.0, std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `string` NUL-terminated; `out_size` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_measure_text_ex(
    ctx: *mut CrabbyCtx,
    string: *const c_char,
    font: CrabbyFont,
    size: f32,
    out_size: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let string = text(string)?;
            if out_size.is_null() {
                return Err(invalid("output is null"));
            }
            if !c.gfx.is_live(font_from_c(font)) {
                return Err(invalid("unknown or freed font"));
            }
            out(out_size, to_c(c.gfx.measure_text_ex(string, font_from_c(font), size)))
        })
    })
}

// ---- sound --------------------------------------------------------------------------

/// Loads a sound (WAV, OGG, MP3 or FLAC) from the assets folder.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_sound(std::ptr::null_mut(), c"a.ogg".as_ptr(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `path` NUL-terminated; `out_sound` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_sound(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_sound: *mut CrabbySound,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_sound.is_null() {
                return Err(invalid("output is null"));
            }
            out(out_sound, sound_to_c(c.load_sound(path)))
        })
    })
}

/// Makes a mono sound from `len` samples in -1..1 at `sample_rate` per second.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_sound(std::ptr::null_mut(), 44100, std::ptr::null(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `samples` must hold `len` floats; `out_sound` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_sound(
    ctx: *mut CrabbyCtx,
    sample_rate: u32,
    samples: *const f32,
    len: usize,
    out_sound: *mut CrabbySound,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if samples.is_null() && len > 0 {
                return Err(invalid("samples is null"));
            }
            if sample_rate == 0 {
                return Err(invalid("the sample rate must not be zero"));
            }
            if out_sound.is_null() {
                return Err(invalid("output is null"));
            }
            let samples = if len == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(samples, len)
            };
            out(out_sound, sound_to_c(c.audio.create(sample_rate, samples)))
        })
    })
}

/// Plays a sound; `params` may be null for the defaults. Writes the playback
/// to `out_playing` unless that is null. Silent in headless runs.
/// ```
/// let s = ponko_ffi::CrabbySound { id: 0, seconds: 0.0 };
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_play_sound(std::ptr::null_mut(), s, std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `params` null or readable; `out_playing` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_play_sound(
    ctx: *mut CrabbyCtx,
    sound: CrabbySound,
    params: *const CrabbySoundParams,
    out_playing: *mut CrabbyPlaying,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let p = if params.is_null() {
                crabby_sound_params_default()
            } else {
                *params
            };
            let playing = c.audio.play_ex(
                &sound_from_c(sound),
                SoundParams {
                    volume: p.volume,
                    speed: p.speed,
                    looping: p.looping,
                },
            );
            if !out_playing.is_null() {
                out_playing.write(playing_to_c(playing));
            }
            Ok(())
        })
    })
}

/// Stops a playback. Stopping one that already ended does nothing.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_stop_sound(std::ptr::null_mut(), Default::default()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stop_sound(ctx: *mut CrabbyCtx, playing: CrabbyPlaying) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.audio.stop(playing_from_c(playing));
            Ok(())
        })
    })
}

// ---- screenshots and turns ---------------------------------------------------------------

/// Saves this frame as a PNG once it is drawn. Does nothing in headless runs.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_save_screenshot(std::ptr::null_mut(), c"a.png".as_ptr()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; `path` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_save_screenshot(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.save_screenshot(text(path)?);
            Ok(())
        })
    })
}

/// Textures whose files changed and were reloaded at the start of this frame
/// (hot reload, see `crabby_game_hot_reload`): `*out_textures` points at
/// `*out_count` of them, valid until the system returns. They keep their
/// handles; use this to redo anything computed from an image.
/// ```
/// let (mut p, mut n) = (std::ptr::null(), 0);
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_reloaded_textures(std::ptr::null(), &mut p, &mut n) } as u32, 1);
/// ```
/// # Safety
/// `ctx` must be null or live; outputs writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reloaded_textures(
    ctx: *const CrabbyCtx,
    out_textures: *mut *const CrabbyTexture,
    out_count: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        if out_textures.is_null() || out_count.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: forwarded caller contract; outputs checked non-null.
        unsafe {
            ctx_access(ctx, |c| {
                let list = c.assets.reloaded_textures();
                // Texture and CrabbyTexture have the same layout (types.rs).
                out_textures.write(list.as_ptr().cast());
                out_count.write(list.len());
                Ok(())
            })
        }
    })
}

/// Spreads work over `every` ticks: see `CrabbyTurns`. `every` must be at least 1.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_turns(std::ptr::null(), 4, std::ptr::null_mut()) } as u32, 1);
/// ```
/// Reads only clock values, so it is safe from any thread during the callback.
/// # Safety
/// `ctx` must be null or live; `out_turns` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_turns(
    ctx: *const CrabbyCtx,
    every: u32,
    out_turns: *mut CrabbyTurns,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        if ctx.is_null() {
            return Err(invalid("ctx is null"));
        }
        if every == 0 {
            return Err(invalid("`every` must be at least 1"));
        }
        let (tick, dt) = (crate::crabby_ctx_sim_tick(ctx), crate::crabby_ctx_dt(ctx));
        out(
            out_turns,
            CrabbyTurns {
                every,
                now: (tick % u64::from(every)) as u32,
                dt: dt * every as f32,
            },
        )
    })
}
