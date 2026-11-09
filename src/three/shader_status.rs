//! What happened to each of the game's shaders: compiled, reloaded or
//! rejected, and where in its WGSL an error is ([`Graphics::shader_status`](crate::Graphics::shader_status)).

use crate::gfx::Graphics;

/// Which kind of WGSL the game gave the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShaderKind {
    /// A material shader ([`Graphics::create_shader`](crate::Graphics::create_shader), [`Graphics::load_shader`](crate::Graphics::load_shader)).
    Material,
    /// A screen effect ([`Graphics::create_effect`](crate::Graphics::create_effect), [`Graphics::load_effect`](crate::Graphics::load_effect)).
    Effect,
    /// A compute pass ([`Graphics::create_compute`](crate::Graphics::create_compute), [`Graphics::load_compute`](crate::Graphics::load_compute)).
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
    Failed,
    /// Its file changed, but the new code has errors: the last good
    /// version stays in use.
    ReloadFailed,
}

/// The latest result of compiling one of the game's shaders, from
/// [`Graphics::shader_status`](crate::Graphics::shader_status).
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
    /// The frame ([`Time::frame`](crate::Time::frame)) this happened in.
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

impl Graphics {
    /// What happened to each of the game's shaders (material shaders,
    /// screen effects, compute passes) the last time it was compiled: one
    /// entry per name and kind, oldest first. A coding agent or tool can
    /// check here whether an edit took effect, and where an error is,
    /// without reading logs.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// for status in ctx.gfx.shader_status() {
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
        let frame = self.frame;
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
                status.outcome = if reload {
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
