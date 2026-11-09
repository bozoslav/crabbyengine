//! What happened to each of the game's shaders: compiled, reloaded or
//! rejected, and where in its WGSL an error is ([`Ctx::shader_status`]).

use crate::Ctx;

/// What a screen effect that can't be built draws instead: the frame as it is.
pub(crate) const EFFECT_STAND_IN: &str = "fn effect(p: Pixel) -> vec3<f32> { return p.color; }";
/// The compute pass a file that doesn't compile gets (it is skipped).
pub(crate) const COMPUTE_STAND_IN: &str = "fn compute(c: Cell) -> vec4<f32> { return c.value; }";

/// Which kind of WGSL the game gave the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShaderKind {
    /// A material shader ([`Ctx::create_shader`], [`Ctx::load_shader`]).
    Material,
    /// A screen effect ([`Ctx::create_effect`], [`Ctx::load_effect`]).
    Effect,
    /// A compute pass ([`Ctx::create_compute`], [`Ctx::load_compute`]).
    Compute,
}

/// A compiler error in one of the game's shaders, pointing at its own WGSL
/// (not at the engine's code the shader is composed with).
///
/// Its `Display` reads like a compiler's: `water.wgsl:4:5: error: ...`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShaderDiagnostic {
    /// The shader's name: the path it was loaded from, or the name given to
    /// `create_shader`.
    pub shader: String,
    /// Material shader, screen effect or compute pass.
    pub kind: ShaderKind,
    /// The variant the engine compiled when it failed: which optional parts
    /// (decals, global illumination, instance storage) the GPU has.
    pub variant: String,
    /// 1-based line of the error in the game's WGSL; in the composed
    /// shader's if `in_engine_code`. `None` if the error has no place.
    pub line: Option<u32>,
    /// 1-based column (in characters) on `line`.
    pub column: Option<u32>,
    /// The error is in the engine's code around the game's: usually the
    /// game's code broke it (an unclosed brace, or a `vertex` / `surface`
    /// hook with the wrong signature).
    pub in_engine_code: bool,
    /// What is wrong, with its causes.
    pub message: String,
    /// The compiler's note on the spot it points at (may be empty).
    pub label: String,
    /// More notes from the compiler.
    pub notes: Vec<String>,
    /// The line of code the error is on.
    pub source_line: Option<String>,
}

impl std::fmt::Display for ShaderDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.shader)?;
        if self.in_engine_code {
            write!(f, " (in the engine's code around it")?;
            if let Some(line) = self.line {
                write!(f, ", composed line {line}")?;
            }
            write!(f, ")")?;
        } else if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": error: {}", self.message)?;
        if let Some(code) = &self.source_line {
            write!(f, "\n    {code}")?;
            let pad = self.column.map_or(0, |c| c.saturating_sub(1) as usize);
            write!(f, "\n    {:pad$}^ {}", "", self.label)?;
        }
        for note in &self.notes {
            write!(f, "\n  = note: {note}")?;
        }
        if !self.variant.is_empty() {
            write!(f, "\n  = variant: {}", self.variant)?;
        }
        Ok(())
    }
}

/// How the last compile of a shader went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShaderOutcome {
    /// Made, and in use.
    Compiled,
    /// Its file changed and the new code is in use.
    Reloaded,
    /// It could not be made (`create_*` returned an error or panicked).
    /// With hot reload on, a shader file that doesn't compile when loaded
    /// still gets its handle: a stand-in (the built-in look for a material
    /// shader, the frame as it is for a screen effect, skipped for a
    /// compute pass) is used, and its file is watched; it stays `Failed`
    /// until a saved fix compiles, which makes it `Reloaded`.
    Failed,
    /// Its file changed, but the new code has errors: the last good
    /// version stays in use.
    ReloadFailed,
}

/// The latest result of compiling one of the game's shaders, from
/// [`Ctx::shader_status`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShaderStatus {
    /// Its name: the path it was loaded from, or the name given to `create_*`.
    pub shader: String,
    /// Material shader, screen effect or compute pass.
    pub kind: ShaderKind,
    /// How it went.
    pub outcome: ShaderOutcome,
    /// How many times an edit of its file was reloaded successfully.
    pub reloads: u32,
    /// The frame ([`Ctx::frame`]) this happened in.
    pub frame: u64,
    /// The errors of the last compile; empty when it worked.
    pub diagnostics: Vec<ShaderDiagnostic>,
}

impl ShaderStatus {
    /// Whether the code in the file is what's in use.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self.outcome, ShaderOutcome::Compiled | ShaderOutcome::Reloaded)
    }
}

impl Ctx {
    /// What happened to each of the game's shaders (material shaders,
    /// screen effects, compute passes) the last time it was compiled: one
    /// entry per name and kind, oldest first. A coding agent or tool can
    /// check here whether an edit took effect, and where an error is,
    /// without reading logs.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// for status in ctx.shader_status() {
    ///     for error in &status.diagnostics {
    ///         eprintln!("{error}"); // water.wgsl:4:5: error: ...
    ///     }
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn shader_status(&self) -> &[ShaderStatus] {
        &self.assets3d.shader_status
    }

    /// Records how compiling shader `name` went; `reload`: a changed file.
    pub(crate) fn record_shader(
        &mut self,
        name: &str,
        kind: ShaderKind,
        reload: bool,
        result: Result<(), Box<ShaderDiagnostic>>,
    ) -> Result<(), String> {
        let frame = self.frame();
        let list = &mut self.assets3d.shader_status;
        let at = match list.iter().position(|s| s.shader == name && s.kind == kind) {
            Some(at) => at,
            None => {
                list.push(ShaderStatus {
                    shader: name.to_string(),
                    kind,
                    outcome: ShaderOutcome::Compiled,
                    reloads: 0,
                    frame,
                    diagnostics: Vec::new(),
                });
                list.len() - 1
            }
        };
        let status = &mut list[at];
        status.frame = frame;
        match result {
            Ok(()) => {
                status.diagnostics.clear();
                status.outcome = if reload {
                    status.reloads += 1;
                    ShaderOutcome::Reloaded
                } else {
                    ShaderOutcome::Compiled
                };
                Ok(())
            }
            Err(diagnostic) => {
                let message = diagnostic.to_string();
                // A file that never compiled has no good version to keep.
                status.outcome = if reload && status.outcome != ShaderOutcome::Failed {
                    ShaderOutcome::ReloadFailed
                } else {
                    ShaderOutcome::Failed
                };
                status.diagnostics = vec![*diagnostic];
                Err(message)
            }
        }
    }
}

impl Ctx {
    /// Whether a shader file that fails when loaded from `origin` gets a
    /// stand-in and stays watched (hot reload on, a file on disk).
    pub(crate) fn keeps_failed_load(&self, origin: &crate::assets::Origin) -> bool {
        self.watcher.on && matches!(origin, crate::assets::Origin::File(_))
    }

    /// Says that `path` failed to compile and uses a stand-in until fixed.
    pub(crate) fn report_stand_in(path: &str, error: &str) {
        eprintln!("ponko: {error}");
        eprintln!("ponko: {path} uses a stand-in until it compiles; hot reload is watching it");
    }

    /// Records shaders whose pipelines the GPU rejected after they were
    /// put in use (a browser's WebGPU answers a frame or so later): the
    /// renderer has put the previous ones back.
    #[cfg(feature = "window")]
    pub(crate) fn apply_shader_checks(&mut self) {
        if self.headless {
            return;
        }
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        // Allocates only when something was rejected.
        let mut late = Vec::new();
        renderer.poll_shader_checks(&mut late);
        for error in late {
            eprintln!("ponko: {} (keeping the previous version)", error.diagnostic);
            self.record_rejected(error.reload, *error.diagnostic);
        }
    }

    /// A shader recorded as compiled turned out to be rejected by the GPU.
    pub(crate) fn record_rejected(&mut self, reload: bool, diagnostic: ShaderDiagnostic) {
        let frame = self.frame();
        let Some(status) = self
            .assets3d
            .shader_status
            .iter_mut()
            .find(|s| s.shader == diagnostic.shader && s.kind == diagnostic.kind)
        else {
            return;
        };
        status.frame = frame;
        if reload {
            if status.outcome == ShaderOutcome::Reloaded {
                status.reloads = status.reloads.saturating_sub(1);
            }
            if status.outcome != ShaderOutcome::Failed {
                status.outcome = ShaderOutcome::ReloadFailed;
            }
        } else {
            status.outcome = ShaderOutcome::Failed;
        }
        status.diagnostics = vec![diagnostic];
    }
}
