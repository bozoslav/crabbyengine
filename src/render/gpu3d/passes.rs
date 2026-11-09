//! What each part of a 3D frame reads and writes, and the rules its order
//! must keep.
//!
//! `Renderer3D::draw` records a frame as a list of parts (see `Part`), each
//! into its own command buffer, on several cores, submitted in list order.
//! This module describes those parts as data: a [`Pass`] names the passes it
//! runs after and the [`Resource`]s it reads and writes. It is a description
//! of the encoder code, not a render graph: it doesn't schedule, allocate or
//! record anything.
//!
//! Two checks use it:
//! - [`check_passes`] checks a table of passes once: names are unique, the
//!   `after` lists form no cycle, and every resource a pass reads is written
//!   only by passes it runs after (a read of last frame's contents, only by
//!   passes that run after it).
//! - [`check_order`] checks one frame's list of parts against the table: no
//!   part comes before a pass it runs after, and every read of a frame
//!   resource follows a part that writes it. `draw` runs it in debug builds,
//!   so every offscreen render test checks the frame it draws. It doesn't
//!   allocate.
//!
//! To add a pass: add its entry to [`FRAME_PASSES`] (with what it runs after,
//! reads and writes), a `Part` for it, and push the part where `draw` builds
//! its list. The tests at the bottom fail if the declarations contradict.

use std::fmt;

/// A GPU resource that passes of a frame share.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Resource {
    /// Skinned models' posed vertices.
    SkinnedVertices,
    /// The GPU-culled draws of static batches, for every view.
    VisibleDraws,
    /// The textures and buffers that compute materials write.
    ComputeTargets,
    /// The sky texture (and its lookup table).
    Sky,
    /// The sun's shadow cascades and the light shadow atlas. Cascades not
    /// redrawn are kept from an earlier frame.
    ShadowMaps,
    /// Target cameras' textures and environment probe captures.
    TargetCameras,
    /// Global illumination's probes.
    GiProbes,
    /// The main camera's (multisampled) depth.
    Depth,
    /// The main camera's (multisampled) color and its resolved lit image.
    SceneColor,
    /// The copy of the scene that transparent surfaces and screen effects read.
    SceneCopy,
    /// Motion vectors of the models that moved.
    Velocity,
    /// Ambient occlusion.
    AmbientOcclusion,
    /// Screen-space reflections traced this frame.
    Reflections,
    /// Last frame's image that screen-space reflections trace against.
    ReflectionHistory,
    /// Temporal antialiasing's accumulated image.
    TaaHistory,
    /// The depth grid read back for CPU occlusion culling.
    OcclusionReadback,
    /// The window or render target the frame ends in.
    Output,
}

/// How long a resource's contents last.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Lifetime {
    /// Written fresh every frame: reading it before a part writes it reads
    /// garbage.
    Frame,
    /// Kept across frames: a frame may read it without writing it.
    Persistent,
    /// Owned outside the 3D renderer.
    External,
}

impl Resource {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Resource::SkinnedVertices => "skinned vertices",
            Resource::VisibleDraws => "visible static draws",
            Resource::ComputeTargets => "compute targets",
            Resource::Sky => "sky texture",
            Resource::ShadowMaps => "shadow maps",
            Resource::TargetCameras => "target cameras",
            Resource::GiProbes => "gi probes",
            Resource::Depth => "scene depth",
            Resource::SceneColor => "scene color",
            Resource::SceneCopy => "scene copy",
            Resource::Velocity => "motion vectors",
            Resource::AmbientOcclusion => "ambient occlusion",
            Resource::Reflections => "reflections",
            Resource::ReflectionHistory => "reflection history",
            Resource::TaaHistory => "taa history",
            Resource::OcclusionReadback => "occlusion readback",
            Resource::Output => "output",
        }
    }

    pub(crate) fn lifetime(self) -> Lifetime {
        match self {
            Resource::SkinnedVertices
            | Resource::VisibleDraws
            | Resource::Depth
            | Resource::SceneColor
            | Resource::SceneCopy
            | Resource::Velocity
            | Resource::AmbientOcclusion
            | Resource::Reflections => Lifetime::Frame,
            Resource::ComputeTargets
            | Resource::Sky
            | Resource::ShadowMaps
            | Resource::TargetCameras
            | Resource::GiProbes
            | Resource::ReflectionHistory
            | Resource::TaaHistory
            | Resource::OcclusionReadback => Lifetime::Persistent,
            Resource::Output => Lifetime::External,
        }
    }
}

/// One pass of a frame (a `Part`), as data.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pass {
    /// Its name, also its command buffer's label.
    pub name: &'static str,
    /// The passes it must be recorded after, when they're in the frame.
    pub after: &'static [&'static str],
    /// What it reads as this frame wrote it.
    pub reads: &'static [Resource],
    /// What it reads as the previous frame left it: passes that write these
    /// must come after it.
    pub reads_previous: &'static [Resource],
    /// What it writes (render attachments, copies, storage).
    pub writes: &'static [Resource],
}

use Resource::*;

/// Index of each part's pass in [`FRAME_PASSES`].
pub(crate) const PRELUDE: usize = 0;
pub(crate) const SHADOW: usize = 1;
pub(crate) const VIEWS: usize = 2;
pub(crate) const GI: usize = 3;
pub(crate) const PREPASS: usize = 4;
pub(crate) const OPAQUE: usize = 5;
pub(crate) const TRANSPARENT: usize = 6;
pub(crate) const POST: usize = 7;

/// The passes of a 3D frame, in the order `draw` records them. Each is
/// present or not depending on the frame (shadow passes repeat: one or more
/// per redrawn cascade).
pub(crate) const FRAME_PASSES: [Pass; 8] = [
    Pass {
        name: "3d prelude",
        after: &[],
        reads: &[],
        reads_previous: &[],
        writes: &[VisibleDraws, SkinnedVertices, ComputeTargets, Sky],
    },
    Pass {
        name: "3d shadows",
        after: &["3d prelude"],
        reads: &[VisibleDraws, SkinnedVertices, ComputeTargets],
        reads_previous: &[],
        writes: &[ShadowMaps],
    },
    Pass {
        name: "3d target cameras",
        after: &["3d prelude", "3d shadows"],
        reads: &[VisibleDraws, SkinnedVertices, ComputeTargets, Sky, ShadowMaps],
        reads_previous: &[],
        writes: &[TargetCameras],
    },
    Pass {
        name: "3d global illumination",
        after: &["3d prelude", "3d shadows"],
        reads: &[VisibleDraws, SkinnedVertices, ComputeTargets, Sky, ShadowMaps],
        reads_previous: &[],
        writes: &[GiProbes],
    },
    Pass {
        name: "3d depth prepass",
        after: &["3d prelude"],
        reads: &[VisibleDraws, SkinnedVertices, ComputeTargets],
        reads_previous: &[ReflectionHistory],
        // The cut-out prepass attaches the scene color (cleared, not kept).
        writes: &[
            Depth,
            SceneColor,
            OcclusionReadback,
            Velocity,
            AmbientOcclusion,
            Reflections,
        ],
    },
    Pass {
        name: "3d opaque",
        after: &[
            "3d prelude",
            "3d shadows",
            "3d target cameras",
            "3d global illumination",
            "3d depth prepass",
        ],
        reads: &[
            VisibleDraws,
            SkinnedVertices,
            ComputeTargets,
            Sky,
            ShadowMaps,
            TargetCameras,
            GiProbes,
            Depth,
            AmbientOcclusion,
            Reflections,
        ],
        reads_previous: &[],
        writes: &[SceneColor, Depth, ReflectionHistory, OcclusionReadback],
    },
    Pass {
        name: "3d transparent",
        after: &["3d opaque"],
        reads: &[
            VisibleDraws,
            SkinnedVertices,
            ComputeTargets,
            Sky,
            ShadowMaps,
            TargetCameras,
            GiProbes,
            Depth,
            SceneColor,
        ],
        reads_previous: &[],
        writes: &[SceneCopy, SceneColor],
    },
    Pass {
        name: "3d post",
        after: &["3d opaque", "3d transparent"],
        reads: &[ComputeTargets, SceneColor, Depth, Velocity],
        reads_previous: &[TaaHistory],
        // Screen effects ping-pong through the scene copy.
        writes: &[TaaHistory, SceneCopy, Output],
    },
];

/// A contradiction in a table of passes or a frame's order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum PassError {
    /// Two passes share a name.
    DuplicateName(&'static str),
    /// A pass runs after one that isn't in the table.
    UnknownPass { pass: &'static str, after: &'static str },
    /// The `after` lists loop: each runs after the next, the last after the first.
    Cycle(Vec<&'static str>),
    /// `pass` reads `resource`, which `writer` writes, but doesn't run after it.
    UndeclaredRead {
        pass: &'static str,
        resource: Resource,
        writer: &'static str,
    },
    /// `pass` reads last frame's `resource`, but `writer` (which overwrites
    /// it) doesn't run after it.
    HistoryOverwritten {
        pass: &'static str,
        resource: Resource,
        writer: &'static str,
    },
    /// Two passes write `resource` with no order between them.
    UnorderedWrites {
        resource: Resource,
        first: &'static str,
        second: &'static str,
    },
    /// In a frame, `pass` comes before `after`, which it must follow.
    OutOfOrder { pass: &'static str, after: &'static str },
    /// In a frame, `pass` reads `resource` before any part wrote it.
    ReadBeforeWrite { pass: &'static str, resource: Resource },
}

impl fmt::Display for PassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PassError::DuplicateName(name) => write!(f, "two passes are named \"{name}\""),
            PassError::UnknownPass { pass, after } => {
                write!(f, "pass \"{pass}\" runs after \"{after}\", which isn't a pass")
            }
            PassError::Cycle(names) => {
                write!(f, "pass dependencies form a cycle: ")?;
                for name in names {
                    write!(f, "\"{name}\" -> ")?;
                }
                write!(f, "\"{}\"", names[0])
            }
            PassError::UndeclaredRead { pass, resource, writer } => write!(
                f,
                "pass \"{pass}\" reads {} but doesn't run after \"{writer}\", which writes it",
                resource.name()
            ),
            PassError::HistoryOverwritten { pass, resource, writer } => write!(
                f,
                "pass \"{pass}\" reads last frame's {} but \"{writer}\", which overwrites it, \
                 doesn't run after it",
                resource.name()
            ),
            PassError::UnorderedWrites {
                resource,
                first,
                second,
            } => write!(
                f,
                "passes \"{first}\" and \"{second}\" both write {} with no order between them",
                resource.name()
            ),
            PassError::OutOfOrder { pass, after } => {
                write!(
                    f,
                    "pass \"{pass}\" is recorded before \"{after}\", which it must follow"
                )
            }
            PassError::ReadBeforeWrite { pass, resource } => write!(
                f,
                "pass \"{pass}\" reads {}, which no earlier pass wrote this frame",
                resource.name()
            ),
        }
    }
}

impl std::error::Error for PassError {}

/// The index of the pass named `name` in `table`.
fn index(table: &[Pass], name: &str) -> Option<usize> {
    table.iter().position(|p| p.name == name)
}

/// Whether pass `later` (transitively) runs after pass `earlier`.
fn runs_after(table: &[Pass], later: usize, earlier: usize) -> bool {
    let mut stack = vec![later];
    let mut seen = vec![false; table.len()];
    while let Some(i) = stack.pop() {
        for &name in table[i].after {
            let Some(j) = index(table, name) else { continue };
            if j == earlier {
                return true;
            }
            if !seen[j] {
                seen[j] = true;
                stack.push(j);
            }
        }
    }
    false
}

/// Checks a table of passes (see the module docs).
pub(crate) fn check_passes(table: &[Pass]) -> Result<(), PassError> {
    for (i, pass) in table.iter().enumerate() {
        if table[..i].iter().any(|p| p.name == pass.name) {
            return Err(PassError::DuplicateName(pass.name));
        }
        for &after in pass.after {
            if index(table, after).is_none() {
                return Err(PassError::UnknownPass { pass: pass.name, after });
            }
        }
    }
    // Cycles: a depth-first walk, with the path on a stack.
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        New,
        OnPath,
        Done,
    }
    fn visit(table: &[Pass], i: usize, marks: &mut [Mark], path: &mut Vec<usize>) -> Result<(), PassError> {
        marks[i] = Mark::OnPath;
        path.push(i);
        for &name in table[i].after {
            let j = index(table, name).unwrap();
            match marks[j] {
                Mark::OnPath => {
                    let start = path.iter().position(|&k| k == j).unwrap();
                    return Err(PassError::Cycle(path[start..].iter().map(|&k| table[k].name).collect()));
                }
                Mark::New => visit(table, j, marks, path)?,
                Mark::Done => {}
            }
        }
        path.pop();
        marks[i] = Mark::Done;
        Ok(())
    }
    let mut marks = vec![Mark::New; table.len()];
    for i in 0..table.len() {
        if marks[i] == Mark::New {
            visit(table, i, &mut marks, &mut Vec::new())?;
        }
    }
    // Hazards, now that `runs_after` terminates.
    for (r, reader) in table.iter().enumerate() {
        for (w, writer) in table.iter().enumerate() {
            if r == w {
                continue;
            }
            for &resource in writer.writes {
                if reader.reads.contains(&resource) && !runs_after(table, r, w) {
                    return Err(PassError::UndeclaredRead {
                        pass: reader.name,
                        resource,
                        writer: writer.name,
                    });
                }
                if reader.reads_previous.contains(&resource) && !runs_after(table, w, r) {
                    return Err(PassError::HistoryOverwritten {
                        pass: reader.name,
                        resource,
                        writer: writer.name,
                    });
                }
                if w > r && reader.writes.contains(&resource) && !runs_after(table, r, w) && !runs_after(table, w, r) {
                    return Err(PassError::UnorderedWrites {
                        resource,
                        first: reader.name,
                        second: writer.name,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Checks one frame's order: `order` lists the frame's parts, as indices in
/// `table`, in the order they're submitted (see the module docs). A pass may
/// repeat (the shadow passes do). Doesn't allocate.
pub(crate) fn check_order<I>(table: &[Pass], order: I) -> Result<(), PassError>
where
    I: Iterator<Item = usize> + Clone,
{
    for (i, part) in order.clone().enumerate() {
        let pass = &table[part];
        for &after in pass.after {
            if order.clone().skip(i + 1).any(|later| table[later].name == after) {
                return Err(PassError::OutOfOrder { pass: pass.name, after });
            }
        }
        for &resource in pass.reads {
            if resource.lifetime() == Lifetime::Frame
                && !order
                    .clone()
                    .take(i)
                    .any(|earlier| earlier != part && table[earlier].writes.contains(&resource))
            {
                return Err(PassError::ReadBeforeWrite {
                    pass: pass.name,
                    resource,
                });
            }
        }
    }
    Ok(())
}

/// The frame's passes, one per line, with what they read and write: for
/// reading a frame's structure in tests and reviews.
#[cfg(test)]
pub(crate) fn describe(table: &[Pass]) -> String {
    let names = |list: &[Resource]| list.iter().map(|r| r.name()).collect::<Vec<_>>().join(", ");
    let mut text = String::new();
    for pass in table {
        text.push_str(&format!(
            "{}: after [{}] reads [{}] reads previous [{}] writes [{}]\n",
            pass.name,
            pass.after.join(", "),
            names(pass.reads),
            names(pass.reads_previous),
            names(pass.writes)
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(name: &'static str, after: &'static [&'static str]) -> Pass {
        Pass {
            name,
            after,
            reads: &[],
            reads_previous: &[],
            writes: &[],
        }
    }

    #[test]
    fn frame_passes_are_consistent() {
        check_passes(&FRAME_PASSES).unwrap();
    }

    #[test]
    fn frame_pass_indices_match_their_names() {
        for (index, name) in [
            (PRELUDE, "3d prelude"),
            (SHADOW, "3d shadows"),
            (VIEWS, "3d target cameras"),
            (GI, "3d global illumination"),
            (PREPASS, "3d depth prepass"),
            (OPAQUE, "3d opaque"),
            (TRANSPARENT, "3d transparent"),
            (POST, "3d post"),
        ] {
            assert_eq!(FRAME_PASSES[index].name, name);
        }
    }

    #[test]
    fn every_frame_order_draw_builds_is_valid() {
        // `draw` always records the prelude, prepass, opaque and post, and
        // optionally shadows (repeated), target cameras, gi and transparent.
        for mask in 0..16u32 {
            for shadow_passes in [0, 1, 3] {
                let mut order = vec![PRELUDE];
                order.extend(std::iter::repeat_n(
                    SHADOW,
                    if mask & 1 != 0 { shadow_passes } else { 0 },
                ));
                if mask & 2 != 0 {
                    order.push(VIEWS);
                }
                if mask & 4 != 0 {
                    order.push(GI);
                }
                order.extend([PREPASS, OPAQUE]);
                if mask & 8 != 0 {
                    order.push(TRANSPARENT);
                }
                order.push(POST);
                check_order(&FRAME_PASSES, order.iter().copied()).unwrap();
            }
        }
    }

    #[test]
    fn frame_passes_are_described() {
        let text = describe(&FRAME_PASSES);
        assert_eq!(text.lines().count(), FRAME_PASSES.len());
        assert!(text.contains(
            "3d transparent: after [3d opaque] reads [visible static draws, skinned vertices, compute targets, \
             sky texture, shadow maps, target cameras, gi probes, scene depth, scene color] reads previous [] \
             writes [scene copy, scene color]"
        ));
    }

    #[test]
    fn a_new_pass_slots_in_with_declared_dependencies() {
        // A pass that reads the opaque scene and writes its own target, for
        // the post pass to read.
        let mut table = FRAME_PASSES.to_vec();
        table.push(Pass {
            name: "3d custom",
            after: &["3d opaque", "3d transparent"],
            reads: &[SceneColor, Depth],
            reads_previous: &[],
            writes: &[TargetCameras],
        });
        let custom = table.len() - 1;
        // Writing target cameras after the opaque pass, which reads them,
        // is caught.
        let error = check_passes(&table).unwrap_err();
        assert_eq!(
            error,
            PassError::UndeclaredRead {
                pass: "3d opaque",
                resource: TargetCameras,
                writer: "3d custom",
            }
        );
        assert_eq!(
            error.to_string(),
            "pass \"3d opaque\" reads target cameras but doesn't run after \"3d custom\", which writes it"
        );
        table[custom].writes = &[Output];
        // `Output` also has the post pass as a writer: order them.
        table[POST].after = &["3d opaque", "3d transparent", "3d custom"];
        check_passes(&table).unwrap();
        let order = [PRELUDE, PREPASS, OPAQUE, custom, POST];
        check_order(&table, order.iter().copied()).unwrap();
        let wrong = [PRELUDE, PREPASS, custom, OPAQUE, POST];
        assert_eq!(
            check_order(&table, wrong.iter().copied()),
            Err(PassError::OutOfOrder {
                pass: "3d custom",
                after: "3d opaque",
            })
        );
    }

    #[test]
    fn cycles_are_rejected_with_their_path() {
        let table = [pass("a", &["c"]), pass("b", &["a"]), pass("c", &["b"])];
        let error = check_passes(&table).unwrap_err();
        assert_eq!(error, PassError::Cycle(vec!["a", "c", "b"]));
        assert_eq!(
            error.to_string(),
            "pass dependencies form a cycle: \"a\" -> \"c\" -> \"b\" -> \"a\""
        );
        let table = [pass("a", &["a"])];
        assert_eq!(check_passes(&table), Err(PassError::Cycle(vec!["a"])));
    }

    #[test]
    fn unknown_and_duplicate_passes_are_rejected() {
        assert_eq!(
            check_passes(&[pass("a", &["missing"])]),
            Err(PassError::UnknownPass {
                pass: "a",
                after: "missing",
            })
        );
        assert_eq!(
            check_passes(&[pass("a", &[]), pass("a", &[])]),
            Err(PassError::DuplicateName("a"))
        );
    }

    #[test]
    fn reads_must_follow_their_writers() {
        let table = [
            Pass {
                writes: &[Depth],
                ..pass("depth", &[])
            },
            Pass {
                reads: &[Depth],
                ..pass("lighting", &[])
            },
        ];
        let error = check_passes(&table).unwrap_err();
        assert_eq!(
            error.to_string(),
            "pass \"lighting\" reads scene depth but doesn't run after \"depth\", which writes it"
        );
    }

    #[test]
    fn writers_of_one_resource_must_be_ordered() {
        let table = [
            Pass {
                writes: &[SceneColor],
                ..pass("a", &[])
            },
            Pass {
                writes: &[SceneColor],
                ..pass("b", &[])
            },
        ];
        assert_eq!(
            check_passes(&table),
            Err(PassError::UnorderedWrites {
                resource: SceneColor,
                first: "a",
                second: "b",
            })
        );
    }

    #[test]
    fn history_must_be_read_before_it_is_overwritten() {
        let table = [
            Pass {
                writes: &[TaaHistory],
                ..pass("resolve", &[])
            },
            Pass {
                reads_previous: &[TaaHistory],
                ..pass("reproject", &["resolve"])
            },
        ];
        assert_eq!(
            check_passes(&table),
            Err(PassError::HistoryOverwritten {
                pass: "reproject",
                resource: TaaHistory,
                writer: "resolve",
            })
        );
    }

    #[test]
    fn frame_orders_reject_reads_of_unwritten_frame_resources() {
        // The opaque pass without the prepass reads a depth no one cleared.
        let order = [PRELUDE, OPAQUE, POST];
        let error = check_order(&FRAME_PASSES, order.iter().copied()).unwrap_err();
        assert_eq!(
            error,
            PassError::ReadBeforeWrite {
                pass: "3d opaque",
                resource: Depth,
            }
        );
        assert_eq!(
            error.to_string(),
            "pass \"3d opaque\" reads scene depth, which no earlier pass wrote this frame"
        );
        // Persistent resources may be read without a writer this frame:
        // shadow cascades kept from an earlier frame.
        check_order(&FRAME_PASSES, [PRELUDE, PREPASS, OPAQUE, POST].into_iter()).unwrap();
    }

    #[test]
    fn frame_orders_reject_passes_before_their_dependencies() {
        let order = [PRELUDE, PREPASS, OPAQUE, POST, TRANSPARENT];
        assert_eq!(
            check_order(&FRAME_PASSES, order.iter().copied()),
            Err(PassError::OutOfOrder {
                pass: "3d post",
                after: "3d transparent",
            })
        );
    }
}
