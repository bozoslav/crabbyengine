//! Material pipeline-set selection (renderer-internal).

/// Which faces of a material's triangles are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MaterialSides {
    /// Back faces culled: the default.
    One,
    /// Both faces drawn (`MaterialDesc::double_sided`); `surface` sees the
    /// normal flipped on the back.
    Both,
}

impl MaterialSides {
    /// Every supported value, in renderer build order.
    pub(crate) const ALL: [MaterialSides; 2] = [MaterialSides::One, MaterialSides::Both];

    /// Resolve the draw flags using the renderer's precedence.
    pub(crate) fn of(double_sided: bool) -> Self {
        if double_sided {
            MaterialSides::Both
        } else {
            MaterialSides::One
        }
    }

    #[cfg(feature = "window")]
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// How a draw's instances reach the vertex shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MaterialInstancing {
    /// One instance record per model (transform, tint, decal layers).
    Plain,
    /// Like `Plain`, for models crossfading between two levels of detail:
    /// every fragment entry point discards the pixels the dither drops.
    Crossfade,
    /// Every instance of the draw reads the same record (zero stride): a
    /// mesh drawn `count` times from a GPU buffer, read as `items` in
    /// `vertex`. Buffer-driven draws never crossfade.
    Shared,
}

impl MaterialInstancing {
    /// Every supported value, in renderer build order.
    #[cfg(test)]
    pub(crate) const ALL: [MaterialInstancing; 3] = [
        MaterialInstancing::Plain,
        MaterialInstancing::Crossfade,
        MaterialInstancing::Shared,
    ];

    /// The instancing of a draw: buffer-driven wins over crossfading.
    /// Resolve the draw flags using the renderer's precedence.
    pub(crate) fn of(fading: bool, shared: bool) -> Self {
        if shared {
            MaterialInstancing::Shared
        } else if fading {
            MaterialInstancing::Crossfade
        } else {
            MaterialInstancing::Plain
        }
    }

    #[cfg(feature = "window")]
    pub(crate) fn fades(self) -> bool {
        self == MaterialInstancing::Crossfade
    }

    #[cfg(feature = "window")]
    pub(crate) fn shared(self) -> bool {
        self == MaterialInstancing::Shared
    }
}

/// Reusable key for a material shader's pipeline set, independent of GPU handles.
///
/// Blend selects a pass inside this set, rather than another set. Skinning
/// happens before drawing; lit/unlit is selected by the surface hook. Shader
/// identity, target formats, samples and device features also enter the
/// renderer's private pipeline cache key. This is not a serialized cache ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MaterialVariantKey {
    /// Which triangle faces are drawn.
    pub(crate) sides: MaterialSides,
    /// How vertex instances are supplied.
    pub(crate) instancing: MaterialInstancing,
}

impl MaterialVariantKey {
    /// Every variant, plain ones first (those are built with the shader).
    /// Every supported value, in renderer build order.
    pub(crate) const ALL: [MaterialVariantKey; 6] = [
        MaterialVariantKey::new(MaterialSides::One, MaterialInstancing::Plain),
        MaterialVariantKey::new(MaterialSides::Both, MaterialInstancing::Plain),
        MaterialVariantKey::new(MaterialSides::One, MaterialInstancing::Crossfade),
        MaterialVariantKey::new(MaterialSides::Both, MaterialInstancing::Crossfade),
        MaterialVariantKey::new(MaterialSides::One, MaterialInstancing::Shared),
        MaterialVariantKey::new(MaterialSides::Both, MaterialInstancing::Shared),
    ];

    /// A key from its two independent features.
    pub(crate) const fn new(sides: MaterialSides, instancing: MaterialInstancing) -> Self {
        Self { sides, instancing }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_keys_cover_the_renderer_sets_without_device_handles() {
        let keys: std::collections::HashSet<_> = MaterialVariantKey::ALL.into_iter().collect();
        assert_eq!(keys.len(), 6);
        for sides in MaterialSides::ALL {
            for instancing in MaterialInstancing::ALL {
                assert!(keys.contains(&MaterialVariantKey::new(sides, instancing)));
            }
        }
        assert_eq!(MaterialInstancing::of(true, true), MaterialInstancing::Shared);
    }
}
