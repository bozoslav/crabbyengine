//! Turns naga's errors about a composed shader (the engine's prelude, the
//! game's code, the engine's entry points) into a [`ShaderDiagnostic`] that
//! points at the line in the game's own WGSL.

use crate::three::{ShaderDiagnostic, ShaderKind};
use wgpu::naga;

/// The line before the game's code in every composed shader.
const GAME_MARKER: &str = "\n// ---- the game's ";
/// The line after it.
const ENGINE_MARKER: &str = "\n// ---- the engine's ";

/// Where the game's code sits in a composed shader: byte range.
fn game_range(composed: &str) -> Option<std::ops::Range<usize>> {
    let marker = composed.find(GAME_MARKER)?;
    // The game's code starts on the line after the marker.
    let start = marker + 1 + composed[marker + 1..].find('\n')? + 1;
    let end = composed[start..]
        .find(ENGINE_MARKER)
        .map_or(composed.len(), |at| start + at);
    Some(start..end)
}

/// Checks composed shader `composed` made from the game's `name`. On an
/// error: where it is in the game's code, and why.
pub(crate) fn diagnose(
    name: &str,
    kind: ShaderKind,
    variant: &str,
    composed: &str,
) -> Result<(), Box<ShaderDiagnostic>> {
    if kind == ShaderKind::Material
        && let Some(game) = game_range(composed)
        && let Some((range, message)) = material_contract(&composed[game.clone()])
    {
        let range = range.start + game.start..range.end + game.start;
        return Err(build(
            name,
            kind,
            variant,
            composed,
            message,
            Some((range, "material contract".into())),
            Vec::new(),
        ));
    }
    let module = match naga::front::wgsl::parse_str(composed) {
        Ok(module) => module,
        Err(e) => {
            let spans: Vec<(std::ops::Range<usize>, String)> = e
                .labels()
                .filter_map(|(span, label)| Some((span.to_range()?, label.to_string())))
                .collect();
            // A parse error's first label is its main one.
            let mut notes: Vec<String> = e.notes().map(str::to_string).collect();
            notes.extend(spans.iter().skip(1).filter(|s| !s.1.is_empty()).map(|s| s.1.clone()));
            let primary = spans.first().cloned();
            return Err(build(
                name,
                kind,
                variant,
                composed,
                e.message().to_string(),
                primary,
                notes,
            ));
        }
    };
    let result = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module);
    if let Err(e) = result {
        // The error, then what caused it, from the outside in.
        let mut message = e.as_inner().to_string();
        let mut source = std::error::Error::source(e.as_inner());
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        let spans: Vec<(std::ops::Range<usize>, String)> = e
            .spans()
            .filter_map(|(span, label)| Some((span.to_range()?, label.clone())))
            .collect();
        // Validation adds the narrowest span first and wraps it in wider
        // ones (the function, ...): point at the narrowest.
        let primary = spans.iter().min_by_key(|(range, _)| range.len()).cloned();
        let notes = spans
            .iter()
            .filter(|s| Some(&s.0) != primary.as_ref().map(|p| &p.0) && !s.1.is_empty())
            .map(|s| s.1.clone())
            .collect();
        return Err(build(name, kind, variant, composed, message, primary, notes));
    }
    Ok(())
}

/// WGSL tokens with byte spans, excluding line and nested block comments.
/// Used before parsing: a bad hook can otherwise fail in the engine's caller.
pub(crate) fn contract_tokens(source: &str) -> Vec<(&str, std::ops::Range<usize>)> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if source[i..].starts_with("//") {
            i += source[i..].find('\n').unwrap_or(source.len() - i);
            continue;
        }
        if source[i..].starts_with("/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if source[i..].starts_with("/*") {
                    depth += 1;
                    i += 2;
                } else if source[i..].starts_with("*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += source[i..].chars().next().unwrap().len_utf8();
                }
            }
            continue;
        }
        let start = i;
        if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
        } else if source[i..].starts_with("->") {
            i += 2;
        } else {
            i += source[i..].chars().next().unwrap().len_utf8();
        }
        tokens.push((&source[start..i], start..i));
    }
    tokens
}

fn material_contract(user: &str) -> Option<(std::ops::Range<usize>, String)> {
    let tokens = contract_tokens(user);
    for (i, (token, span)) in tokens.iter().enumerate() {
        if *token == "@"
            && tokens
                .get(i + 1)
                .is_some_and(|t| matches!(t.0, "vertex" | "fragment" | "compute"))
        {
            return Some((span.clone(), "material hooks `vertex`/`surface` are ordinary functions, not shader-stage entry points; expected `fn vertex(value: Vertex) -> Vertex` or `fn surface(value: Surface) -> Surface`; remove the @vertex/@fragment/@compute attribute and let the engine provide the entry points".into()));
        }
        if *token == "@" && tokens.get(i + 1).is_some_and(|t| matches!(t.0, "group" | "binding")) {
            return Some((span.clone(), "material hooks `vertex`/`surface` may not declare bindings: every @group/@binding is engine-owned; remove the declaration and use material.custom, custom_texture0..1, custom_array0..2 or items through the engine API".into()));
        }
        if *token != "fn" {
            continue;
        }
        let Some((hook, _)) = tokens.get(i + 1) else {
            continue;
        };
        let ty = match *hook {
            "vertex" => "Vertex",
            "surface" => "Surface",
            _ => continue,
        };
        let signature: Vec<_> = tokens
            .iter()
            .skip(i + 2)
            .take_while(|t| t.0 != "{")
            .map(|t| t.0)
            .collect();
        // Parameter names are chosen by the caller. A trailing comma is WGSL.
        let valid = signature.len() >= 7
            && signature[0] == "("
            && signature[2] == ":"
            && signature[3] == ty
            && (signature[4..] == [")", "->", ty] || signature[4..] == [",", ")", "->", ty]);
        if !valid {
            return Some((
                span.clone(),
                format!(
                    "material hook `{hook}` has the wrong signature; expected `fn {hook}(value: {ty}) -> {ty}`; replace its parameters and return type with this signature (or remove the hook for the pass-through default)"
                ),
            ));
        }
    }
    None
}

/// An unsupported material/storage combination, located in the user hook.
/// Comments do not count as reads of the engine's `items` buffer.
pub(crate) fn require_material_storage(name: &str, variant: &str, user: &str) -> Result<(), Box<ShaderDiagnostic>> {
    let Some((_, span)) = contract_tokens(user).into_iter().find(|t| t.0 == "items") else {
        return Ok(());
    };
    let composed = crate::render::compose_shader(user);
    let game = game_range(&composed).expect("material source marker");
    Err(build(
        name, ShaderKind::Material, variant, &composed,
        "material hooks `vertex`/`surface` may read `items` only with vertex storage buffers; this GPU has none; use create_instance_batch or Model entities with a shader that does not read items, or select a storage-capable GPU".into(),
        Some((span.start + game.start..span.end + game.start, "unsupported instance storage read".into())), Vec::new(),
    ))
}

/// A diagnostic for an error that isn't the compiler's (pipeline creation),
/// with no place in the source.
pub(crate) fn without_location(name: &str, kind: ShaderKind, variant: &str, message: String) -> Box<ShaderDiagnostic> {
    Box::new(ShaderDiagnostic {
        shader: name.to_string(),
        kind,
        variant: variant.to_string(),
        line: None,
        column: None,
        in_engine_code: false,
        message,
        label: String::new(),
        notes: Vec::new(),
        source_line: None,
    })
}

fn build(
    name: &str,
    kind: ShaderKind,
    variant: &str,
    composed: &str,
    message: String,
    primary: Option<(std::ops::Range<usize>, String)>,
    notes: Vec<String>,
) -> Box<ShaderDiagnostic> {
    let mut diagnostic = without_location(name, kind, variant, message);
    diagnostic.notes = notes;
    let Some((range, label)) = primary else {
        return diagnostic;
    };
    diagnostic.label = label;
    let at = range.start.min(composed.len());
    let line_start = composed[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = composed[at..].find('\n').map_or(composed.len(), |i| at + i);
    let column = composed[line_start..at].chars().count() as u32 + 1;
    diagnostic.source_line = Some(composed[line_start..line_end].trim_end().to_string());
    match game_range(composed) {
        Some(game) if game.contains(&at) || (at == game.end && !game.is_empty()) => {
            let line = composed[game.start..line_start].matches('\n').count() as u32 + 1;
            diagnostic.line = Some(line);
            diagnostic.column = Some(column);
        }
        _ => {
            // In the engine's code: often the game's code broke what came
            // after it (an unclosed brace), or a hook has the wrong type.
            diagnostic.in_engine_code = true;
            diagnostic.line = Some(composed[..line_start].matches('\n').count() as u32 + 1);
            diagnostic.column = Some(column);
        }
    }
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{compose_compute, compose_effect, compose_shader};

    #[test]
    fn parse_errors_point_at_the_games_line_and_column() {
        let user = "fn surface(s: Surface) -> Surface {\n    var out = s;\n    out.albedo = vec3<f32>(1.0, 0.0, 0.0)\n    return out;\n}";
        let d = diagnose("red.wgsl", ShaderKind::Material, "material", &compose_shader(user)).unwrap_err();
        assert_eq!(d.shader, "red.wgsl");
        assert!(!d.in_engine_code, "{d}");
        // The missing `;`: naga points at what follows it.
        assert!(matches!(d.line, Some(3 | 4)), "{d}");
        assert!(d.column.is_some(), "{d}");
        assert!(!d.message.is_empty());
        assert!(
            d.to_string().starts_with(&format!("red.wgsl:{}:", d.line.unwrap())),
            "{d}"
        );
    }

    #[test]
    fn validation_errors_point_at_the_games_line_with_the_cause() {
        let user = "// a comment\n\nfn surface(s: Surface) -> Surface {\n    return 1.0;\n}";
        let d = diagnose("water.wgsl", ShaderKind::Material, "material", &compose_shader(user)).unwrap_err();
        assert!(!d.in_engine_code, "{d}");
        assert_eq!(d.line, Some(4), "{d}");
        assert_eq!(d.column, Some(12), "{d}");
        assert_eq!(d.source_line.as_deref(), Some("    return 1.0;"));
        assert!(d.message.contains("Surface"), "{d}");
        assert!(
            d.to_string().contains("\n        return 1.0;\n               ^ "),
            "{d}"
        );
    }

    #[test]
    fn validator_errors_have_their_causes() {
        // Parses, but the validator rejects it: a function that never returns.
        let user = "fn helper() -> f32 {\n}\nfn surface(s: Surface) -> Surface { return s; }";
        let d = diagnose("v.wgsl", ShaderKind::Material, "material", &compose_shader(user)).unwrap_err();
        assert!(!d.message.is_empty(), "{d}");
        assert!(d.message.contains(": "), "the cause follows: {d}");
        assert!(d.line.is_some(), "{d}");
    }

    #[test]
    fn effects_and_compute_passes_map_to_their_own_lines() {
        let effect = "fn effect(p: Pixel) -> vec4<f32> {\n    return p.color + nope;\n}";
        let d = diagnose("fx.wgsl", ShaderKind::Effect, "effect", &compose_effect(effect)).unwrap_err();
        assert_eq!((d.line, d.in_engine_code), (Some(2), false), "{d}");
        let compute = "\nfn compute(c: Cell) -> vec4<f32> {\n    return undefined_thing;\n}";
        let d = diagnose("sim.wgsl", ShaderKind::Compute, "compute", &compose_compute(compute)).unwrap_err();
        assert_eq!((d.line, d.in_engine_code), (Some(3), false), "{d}");
    }

    #[test]
    fn valid_shaders_have_no_diagnostic() {
        diagnose("ok", ShaderKind::Material, "material", &compose_shader("")).unwrap();
    }

    #[test]
    fn the_game_range_is_exactly_the_games_code() {
        let user = "fn surface(s: Surface) -> Surface { return s; }";
        let composed = compose_shader(user);
        let range = game_range(&composed).unwrap();
        assert_eq!(&composed[range], user);
    }
}
