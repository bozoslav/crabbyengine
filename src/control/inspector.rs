//! `perf.get`'s `render_inspector`: one frame's rendering cost in a fixed,
//! versioned shape for tools and agents.
//!
//! Every metric has a stable field name whose suffix gives its unit (`_ms`
//! milliseconds, `_bytes` bytes; counts are per frame). A metric the engine
//! can't measure here is `null`, never `0`, and `unavailable` maps its path
//! (`"gpu_ms.frame"`) to the reason. In particular GPU times are only ever
//! reported from timestamp queries: an adapter without them gets `null`.
//!
//! Built only when asked for, from numbers the renderer already keeps, so
//! it adds nothing to a frame.

use serde_json::{Map, Value, json};

use crate::perf::GpuPass;
use crate::render::PipelineStats;
use crate::render::{RenderDetail, RenderStats};

/// The `render_inspector` schema version. Bumped only when a field is
/// removed or changes meaning; new fields don't bump it.
pub(super) const SCHEMA: u32 = 1;

/// Where the frame numbers come from.
#[derive(Debug, Clone, Copy)]
pub(super) enum Source {
    /// A headless run: nothing is drawn.
    Headless,
    /// A windowed game that hasn't drawn a frame yet (or has no renderer).
    NotDrawn,
    /// The renderer drew a frame.
    Drawn(RenderDetail),
}

pub(super) struct Inputs<'a> {
    pub source: Source,
    pub stats: RenderStats,
    /// Smoothed GPU time per pass, in the order they run.
    pub passes: &'a [GpuPass],
    /// The active graphics quality settings, if the build has them.
    pub quality: Option<Value>,
    /// The render pipeline cache since the renderer started.
    pub pipelines: PipelineStats,
    /// What the renderer's own tables hold on the GPU; `None` without one.
    pub memory: Option<crate::inventory::GpuResidency>,
    /// The GPU the renderer opened; `None` without one.
    pub adapter: Option<&'a crate::render::GpuCapabilities>,
    /// Recent drawn frames, oldest first: as many as the caller asked for.
    pub history: &'a [crate::perf::RenderFrame],
    /// Plain models outside the camera's view in the last frame, or why
    /// they can't be counted now.
    pub frustum_culled: Result<usize, &'static str>,
}

const NO_TIMESTAMPS: &str = "the GPU adapter doesn't support timestamp queries, so GPU time can't be measured";
const NOT_TIMED_YET: &str = "no frame's GPU timestamps have come back yet";
const NO_GPU: &str = "headless runs open no GPU";
const NO_GPU_YET: &str = "the renderer hasn't opened a GPU yet";
/// Why `counts.frustum_culled_other_models` is always null.
const FRUSTUM_OTHER: &str = "not counted: LOD groups, terrain tiles, skinned models, models with morph targets and shadow-only casters are culled anew every frame without a result kept to count from, and static instance batches are culled on the GPU (see counts.instance_batches); counts.frustum_culled_models covers plain models";
/// Why `counts.frustum_culled_models` is null in a rush.
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
pub(super) const FRUSTUM_STALE: &str = "most models changed in recent frames, so what is kept per model is stale and isn't counted from; it is again once they settle";

pub(super) fn render_inspector(inputs: &Inputs<'_>) -> Value {
    let mut unavailable = Map::new();
    let mut gone = |path: &str, why: &str| {
        unavailable.insert(path.into(), json!(why));
    };
    let no_gpu = match inputs.source {
        Source::Headless => NO_GPU,
        _ => NO_GPU_YET,
    };
    // What the renderer's tables hold: its textures (render targets
    // included), mesh buffers, GPU buffers and the instance batches' pools.
    let memory = match &inputs.memory {
        Some(m) => {
            let pools = inputs.stats.instance_batches.pool_bytes;
            json!({
                "total": m.texture_bytes + m.mesh_bytes + m.gpu_buffer_bytes + pools,
                "textures": m.texture_bytes,
                "meshes": m.mesh_bytes,
                "gpu_buffers": m.gpu_buffer_bytes,
                "instance_pools": pools,
            })
        }
        None => {
            gone("memory_estimate_bytes", no_gpu);
            Value::Null
        }
    };
    let adapter = match inputs.adapter {
        Some(caps) => json!({
            "name": caps.adapter.name,
            "backend": caps.adapter.backend,
            "device_type": caps.adapter.device_type,
            "driver": caps.adapter.driver,
            "driver_info": caps.adapter.driver_info,
            "tier": caps.tier,
            "forced": caps.forced_tier.is_some(),
        }),
        None => {
            gone("adapter", no_gpu);
            Value::Null
        }
    };
    let history: Vec<Value> = (inputs.history.iter())
        .map(|f| {
            json!({
                "frame": f.frame,
                "cpu_ms": f.cpu_ms,
                "gpu_smoothed_ms": f.gpu_smoothed_ms,
                "gpu_latest_ms": f.gpu_latest.map(|(ms, _)| ms),
                "gpu_latest_frames_ago": f.gpu_latest.map(|(_, ago)| ago),
                "draw_calls": f.draw_calls,
                "models": f.models,
                "triangles": f.triangles,
                "shadow_triangles": f.shadow_triangles,
                "upload_bytes": f.upload_bytes,
            })
        })
        .collect();

    let (gpu_timing, cpu_ms, gpu_ms, counts, upload_bytes) = match inputs.source {
        Source::Headless | Source::NotDrawn => {
            let why = match inputs.source {
                Source::Headless => "headless runs draw nothing",
                _ => "no frame has been drawn yet",
            };
            for path in ["cpu_ms", "gpu_ms", "counts", "upload_bytes"] {
                gone(path, why);
            }
            let timing = json!({ "available": false, "frames_timed": 0 });
            (timing, Value::Null, Value::Null, Value::Null, Value::Null)
        }
        Source::Drawn(detail) => {
            let s = &inputs.stats;
            // The renderer's whole CPU time and its measured parts; what's
            // left (sprite batching, uploading sprites, overlays) is `other`.
            let extract = s.collect_ms;
            let prepare = (detail.prepare_ms - s.collect_ms).max(0.0);
            let other = (s.cpu_ms - extract - prepare - s.record_ms - s.submit_ms).max(0.0);
            let cpu = json!({
                "total": s.cpu_ms,
                "extract": extract,
                "prepare": prepare,
                "encode": s.record_ms,
                "submit": s.submit_ms,
                "other": other,
            });
            let timed = detail.gpu_timestamps && detail.gpu_frames_timed > 0;
            let (frame, passes) = if !detail.gpu_timestamps {
                gone("gpu_ms.frame", NO_TIMESTAMPS);
                gone("gpu_ms.passes", NO_TIMESTAMPS);
                (Value::Null, Value::Null)
            } else if !timed {
                gone("gpu_ms.frame", NOT_TIMED_YET);
                gone("gpu_ms.passes", NOT_TIMED_YET);
                (Value::Null, Value::Null)
            } else {
                let passes: Vec<Value> = inputs
                    .passes
                    .iter()
                    .map(|p| json!({ "name": p.name, "ms": p.ms }))
                    .collect();
                let frame = match s.gpu_ms {
                    Some(ms) => json!(ms),
                    None => {
                        gone("gpu_ms.frame", NOT_TIMED_YET);
                        Value::Null
                    }
                };
                (frame, Value::Array(passes))
            };
            let frustum_culled = match inputs.frustum_culled {
                Ok(n) => json!(n),
                Err(why) => {
                    gone("counts.frustum_culled_models", why);
                    Value::Null
                }
            };
            gone("counts.frustum_culled_other_models", FRUSTUM_OTHER);
            if detail.state_changes.is_none() {
                gone("counts.state_changes", "3D model passes are not enabled");
            }
            let counts = json!({
                "state_changes": crate::render::state_changes_json(detail.state_changes),
                "draw_calls": s.draw_calls,
                "sprites": s.sprites,
                "models": s.instances,
                "triangles": s.triangles,
                "shadow_models": s.shadow_instances,
                "shadow_triangles": s.shadow_triangles,
                "occluded_models": s.occluded,
                "frustum_culled_models": frustum_culled,
                "frustum_culled_other_models": null,
                "pipelines_built": s.pipelines_built,
                "instance_batches": instance_batches_json(&s.instance_batches),
            });
            let timing = json!({
                "available": detail.gpu_timestamps,
                "frames_timed": detail.gpu_frames_timed,
            });
            let uploads = json!(s.uploads.bytes());
            (
                timing,
                cpu,
                json!({ "frame": frame, "passes": passes }),
                counts,
                uploads,
            )
        }
    };
    let pipelines = if let Source::Headless = inputs.source {
        gone("pipelines", "headless runs build no pipelines");
        Value::Null
    } else {
        pipelines_json(&inputs.pipelines)
    };
    let quality = match &inputs.quality {
        Some(q) => q.clone(),
        None => {
            gone("quality", "graphics quality settings need the `3d` feature");
            Value::Null
        }
    };
    json!({
        "schema": SCHEMA,
        "units": {
            "_ms": "milliseconds; CPU times are the last drawn frame's, GPU times are smoothed over recent frames",
            "_bytes": "bytes",
            "counts": "per frame",
            "memory_estimate_bytes": "what the renderer's tables hold for the game: its textures (render-target textures included), mesh buffers, its GPU buffers and the instance batch pools; not the frame's own targets (scene color and depth, post-processing, ambient occlusion and temporal history), shadow maps, the light shadow atlas, skinning or per-frame instance buffers; an estimate, not the driver's total",
            "occluded_models": "models in the camera's view that occlusion culling hid in the last frame: plain models, and each LOD renderer once (a crossfade between two hidden levels counts both); shadow-only casters are not counted",
            "frustum_culled_models": "plain Model entities (not LOD groups, terrain tiles, skinned models, models with morph targets, shadow-only casters or static instance batches) whose bounds were outside the camera's view in the last frame, counted when asked from what the renderer keeps per model; models hidden by occlusion culling are in the view and not counted here (see occluded_models)",
            "history": "one entry per drawn frame, oldest first. gpu_smoothed_ms: the smoothed GPU time as it stood after that frame. gpu_latest_ms: the newest single frame's own GPU time that had come back by then, from gpu_latest_frames_ago drawn frames earlier (timestamps come back late, so a frame's own GPU time isn't known when it is kept). Both are null when GPU time wasn't measured",
        },
        "adapter": adapter,
        "gpu_timing": gpu_timing,
        "cpu_ms": cpu_ms,
        "gpu_ms": gpu_ms,
        "counts": counts,
        "upload_bytes": upload_bytes,
        "memory_estimate_bytes": memory,
        "history": history,
        "pipelines": pipelines,
        "quality": quality,
        "unavailable": unavailable,
    })
}

/// How the static instance batches were culled and drawn in the frame:
/// [`InstanceBatchStats`](crate::render::InstanceBatchStats), field by field.
fn instance_batches_json(b: &crate::render::InstanceBatchStats) -> Value {
    json!({
        "batches": b.batches,
        "instances": b.instances,
        "gpu_driven": b.gpu_driven,
        "gpu_instances": b.gpu_instances,
        "cpu_small": b.cpu_small,
        "cpu_no_compute": b.cpu_no_compute,
        "cpu_over_limit": b.cpu_over_limit,
        "dropped": b.dropped,
        "pools": b.pools,
        "pool_bytes": b.pool_bytes,
        "cull_dispatches": b.cull_dispatches,
        "cull_jobs": b.cull_jobs,
        "views_skipped": b.views_skipped,
        "pages": b.pages,
    })
}

/// The pipeline cache's totals since the renderer started (not per frame).
fn pipelines_json(p: &PipelineStats) -> Value {
    json!({
        "material_shaders": p.material_shaders,
        "variant_sets": p.variant_sets,
        "pipelines": p.pipelines,
        "hits": p.hits,
        "misses": p.misses,
        "shared": p.shared,
        "builds": p.builds,
        "build_ms_total": p.build_ms,
        "slowest_build_ms": p.slowest_build_ms,
        "compiles": p.compiles,
        "compile_ms_total": p.compile_ms,
        "slowest_compile_ms": p.slowest_compile_ms,
        "last_compile_ms": p.last_compile_ms,
        "invalidated": p.invalidated,
        "key_bytes": p.key_bytes,
    })
}

/// The active graphics quality: its preset level and every setting.
#[cfg(feature = "3d")]
pub(super) fn quality_json(q: &crate::render::GraphicsQuality) -> Value {
    let level = match q.level() {
        crate::render::QualityLevel::Low => "low",
        crate::render::QualityLevel::Medium => "medium",
        crate::render::QualityLevel::High => "high",
        crate::render::QualityLevel::Ultra => "ultra",
        crate::render::QualityLevel::Custom => "custom",
    };
    json!({
        "level": level,
        "render_scale": q.render_scale,
        "sharpen": q.sharpen,
        "shadows": q.shadows,
        "shadow_distance": q.shadow_distance,
        "shadow_cascades": q.shadow_cascades,
        "shadow_resolution": q.shadow_resolution,
        "shadow_refresh": q.shadow_refresh,
        "shadow_filter": q.shadow_filter,
        "light_shadow_views": q.light_shadow_views,
        "light_shadow_resolution": q.light_shadow_resolution,
        "light_shadow_atlas": q.light_shadow_atlas,
        "ambient_occlusion": q.ambient_occlusion,
        "ao_half_resolution": q.ao_half_resolution,
        "ao_temporal": q.ao_temporal,
        "ao_slices": q.ao_slices,
        "ao_steps": q.ao_steps,
        "ssr": q.ssr,
        "gi": q.gi,
        "anti_aliasing": format!("{:?}", q.anti_aliasing).to_lowercase(),
    })
}

/// Every null field outside arrays has a reason in `unavailable`, and
/// every reason names a null field (also used on `perf.get`'s whole answer).
#[cfg(test)]
#[track_caller]
pub(super) fn assert_nulls_explained(v: &Value) {
    let unavailable = v["unavailable"].as_object().expect("unavailable is a map");
    for (path, why) in unavailable {
        let mut at = v;
        for part in path.split('.') {
            at = &at[part];
        }
        assert!(at.is_null(), "{path} has a reason but a value: {v}");
        assert!(why.as_str().is_some_and(|w| !w.is_empty()), "{path}: {why}");
    }
    fn walk(v: &Value, path: &str, unavailable: &Map<String, Value>) {
        match v {
            Value::Null => assert!(unavailable.contains_key(path), "{path} is null without a reason"),
            Value::Object(m) if path != "unavailable" && path != "units" => {
                for (k, x) in m {
                    let p = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    walk(x, &p, unavailable);
                }
            }
            _ => {}
        }
    }
    walk(v, "", unavailable);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(gpu_timestamps: bool, gpu_frames_timed: u64) -> Source {
        Source::Drawn(RenderDetail {
            gpu_timestamps,
            gpu_frames_timed,
            prepare_ms: 1.5,
            state_changes: Some([2, 7, 3, 2, 11]),
            gpu_latest: None,
        })
    }

    fn stats(gpu_ms: Option<f32>) -> RenderStats {
        RenderStats {
            sprites: 3,
            draw_calls: 7,
            cpu_ms: 4.0,
            gpu_ms,
            collect_ms: 0.5,
            record_ms: 1.25,
            submit_ms: 0.25,
            instances: 10,
            triangles: 1200,
            shadow_instances: 4,
            shadow_triangles: 300,
            occluded: 2,
            pipelines_built: 0,
            uploads: crate::render::GpuUploads {
                instances: 640,
                uniforms: 256,
                textures: 4096,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    const PASSES: [GpuPass; 2] = [
        GpuPass {
            name: "shadows 0",
            ms: 0.4,
        },
        GpuPass {
            name: "opaque",
            ms: 1.1,
        },
    ];

    const HISTORY: [crate::perf::RenderFrame; 2] = [
        crate::perf::RenderFrame {
            frame: 10,
            cpu_ms: 3.0,
            gpu_smoothed_ms: None,
            gpu_latest: None,
            draw_calls: 6,
            models: 9,
            triangles: 1100,
            shadow_triangles: 300,
            upload_bytes: 512,
        },
        crate::perf::RenderFrame {
            frame: 11,
            cpu_ms: 4.0,
            gpu_smoothed_ms: Some(2.0),
            gpu_latest: Some((2.75, 3)),
            draw_calls: 7,
            models: 10,
            triangles: 1200,
            shadow_triangles: 300,
            upload_bytes: 64,
        },
    ];

    fn inspect(source: Source, gpu_ms: Option<f32>) -> Value {
        render_inspector(&Inputs {
            source,
            stats: stats(gpu_ms),
            passes: &PASSES,
            quality: Some(json!({ "level": "high" })),
            pipelines: PipelineStats {
                misses: 2,
                hits: 40,
                ..Default::default()
            },
            memory: matches!(source, Source::Drawn(_)).then(|| crate::inventory::GpuResidency {
                textures: 3,
                texture_bytes: 4096,
                meshes: 2,
                mesh_bytes: 1000,
                gpu_buffers: 1,
                gpu_buffer_bytes: 24,
            }),
            adapter: None,
            history: &HISTORY,
            frustum_culled: Ok(5),
        })
    }

    /// Negative control for [`assert_nulls_explained`]: a null field with
    /// no `unavailable` entry fails it. The check walks objects only:
    /// arrays are skipped, so a `null` inside `history[]` (an entry's GPU
    /// times when they weren't measured) is explained by the `units` text
    /// for `history`, not by `unavailable`.
    #[test]
    #[should_panic(expected = "counts.frustum_culled_models is null without a reason")]
    fn a_null_without_a_reason_fails_the_check() {
        let mut v = inspect(drawn(true, 5), Some(2.0));
        // Arrays are skipped: this alone passes.
        v["history"][0]["gpu_smoothed_ms"] = Value::Null;
        assert_nulls_explained(&v);
        v["counts"]["frustum_culled_models"] = Value::Null;
        assert_nulls_explained(&v);
    }

    #[test]
    fn the_schema_has_stable_names() {
        let v = inspect(drawn(true, 5), Some(2.0));
        assert_eq!(v["schema"], SCHEMA);
        for key in [
            "units",
            "gpu_timing",
            "cpu_ms",
            "gpu_ms",
            "counts",
            "upload_bytes",
            "memory_estimate_bytes",
            "adapter",
            "history",
            "pipelines",
            "quality",
            "unavailable",
        ] {
            assert!(v.get(key).is_some(), "{key} missing from {v}");
        }
        assert_eq!(
            v["upload_bytes"],
            stats(None).uploads.bytes(),
            "a drawn frame counts all its uploads: {v}"
        );
        assert_eq!(v["upload_bytes"], 640 + 256 + 4096);
        assert_eq!(v["pipelines"]["misses"], 2, "pipeline cache totals: {v}");
        assert_eq!(v["pipelines"]["hits"], 40);
        for key in ["total", "extract", "prepare", "encode", "submit", "other"] {
            assert!(v["cpu_ms"][key].is_number(), "cpu_ms.{key}: {v}");
        }
        for key in [
            "draw_calls",
            "sprites",
            "models",
            "triangles",
            "shadow_models",
            "shadow_triangles",
            "occluded_models",
            "pipelines_built",
        ] {
            assert!(v["counts"][key].is_u64(), "counts.{key}: {v}");
        }
        assert_nulls_explained(&v);
    }

    #[test]
    fn memory_is_the_sum_of_what_the_renderer_holds() {
        let v = inspect(drawn(true, 5), Some(2.0));
        let m = &v["memory_estimate_bytes"];
        assert_eq!((m["textures"].as_u64(), m["meshes"].as_u64()), (Some(4096), Some(1000)));
        assert_eq!(
            (m["gpu_buffers"].as_u64(), m["instance_pools"].as_u64()),
            (Some(24), Some(0))
        );
        assert_eq!(m["total"], 4096 + 1000 + 24);
        assert!(
            v["units"]["memory_estimate_bytes"]
                .as_str()
                .unwrap()
                .contains("estimate")
        );
    }

    #[test]
    fn history_lists_the_frames_it_was_given_oldest_first() {
        let v = inspect(drawn(true, 5), Some(2.0));
        let frames = v["history"].as_array().unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(
            (frames[0]["frame"].as_u64(), frames[1]["frame"].as_u64()),
            (Some(10), Some(11))
        );
        // A frame whose GPU time wasn't measured says so with null, not 0.
        for key in ["gpu_smoothed_ms", "gpu_latest_ms", "gpu_latest_frames_ago"] {
            assert!(frames[0][key].is_null(), "history.{key}: {v}");
        }
        // The smoothed time and the newest single frame's, with its age.
        assert_eq!(frames[1]["gpu_smoothed_ms"], 2.0);
        assert_eq!(
            (
                frames[1]["gpu_latest_ms"].clone(),
                frames[1]["gpu_latest_frames_ago"].clone()
            ),
            (json!(2.75), json!(3))
        );
        for key in [
            "cpu_ms",
            "draw_calls",
            "models",
            "triangles",
            "shadow_triangles",
            "upload_bytes",
        ] {
            assert!(frames[1][key].is_number(), "history.{key}: {v}");
        }
    }

    #[test]
    fn frustum_culled_models_are_counted_or_explained() {
        let v = inspect(drawn(true, 5), Some(2.0));
        assert_eq!(v["counts"]["frustum_culled_models"], 5);
        assert!(v["unavailable"].get("counts.frustum_culled_models").is_none());
        // What isn't counted is named, with the reason.
        assert!(v["counts"]["frustum_culled_other_models"].is_null());
        let other = v["unavailable"]["counts.frustum_culled_other_models"].as_str().unwrap();
        assert!(other.contains("LOD") && other.contains("GPU"), "{other}");
        assert!(v["units"]["frustum_culled_models"].as_str().unwrap().contains("plain"));
        assert_nulls_explained(&v);

        let stale = render_inspector(&Inputs {
            source: drawn(true, 5),
            stats: stats(Some(2.0)),
            passes: &PASSES,
            quality: None,
            pipelines: PipelineStats::default(),
            memory: None,
            adapter: None,
            history: &[],
            frustum_culled: Err(FRUSTUM_STALE),
        });
        assert!(stale["counts"]["frustum_culled_models"].is_null());
        assert_eq!(stale["unavailable"]["counts.frustum_culled_models"], FRUSTUM_STALE);
        assert_nulls_explained(&stale);
    }

    #[test]
    fn the_adapter_is_named_with_its_tier() {
        let caps = crate::render::GpuTier::Baseline.report();
        let v = render_inspector(&Inputs {
            source: drawn(false, 0),
            stats: stats(None),
            passes: &[],
            quality: None,
            pipelines: PipelineStats::default(),
            memory: None,
            adapter: Some(&caps),
            history: &[],
            frustum_culled: Ok(0),
        });
        let a = &v["adapter"];
        for key in ["name", "backend", "device_type", "driver", "driver_info", "tier"] {
            assert!(a[key].is_string(), "adapter.{key}: {v}");
        }
        assert_eq!(a["tier"], "baseline");
        assert_eq!(a["forced"], false);
        assert_nulls_explained(&v);
    }

    #[test]
    fn cpu_parts_add_up_to_the_total() {
        let v = inspect(drawn(true, 5), Some(2.0));
        let cpu = &v["cpu_ms"];
        assert_eq!(cpu["extract"], 0.5);
        assert_eq!(cpu["prepare"], 1.0, "prepare excludes extract: {cpu}");
        assert_eq!(cpu["encode"], 1.25);
        assert_eq!(cpu["submit"], 0.25);
        let parts: f64 = ["extract", "prepare", "encode", "submit", "other"]
            .iter()
            .map(|k| cpu[k].as_f64().unwrap())
            .sum();
        assert!((parts - cpu["total"].as_f64().unwrap()).abs() < 1e-6, "{cpu}");
    }

    #[test]
    fn measured_gpu_times_are_reported_per_pass_in_order() {
        let v = inspect(drawn(true, 5), Some(2.0));
        assert_eq!(v["gpu_timing"], json!({ "available": true, "frames_timed": 5 }));
        assert_eq!(v["gpu_ms"]["frame"], 2.0);
        let names: Vec<_> = v["gpu_ms"]["passes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].clone())
            .collect();
        assert_eq!(names, [json!("shadows 0"), json!("opaque")]);
        assert!(v["unavailable"].get("gpu_ms.frame").is_none());
    }

    #[test]
    fn adapters_without_timestamps_get_null_gpu_times_not_zero() {
        // Even if stale numbers were lying around, nothing is made up.
        let v = inspect(drawn(false, 0), Some(0.0));
        assert_eq!(v["gpu_timing"]["available"], false);
        assert!(v["gpu_ms"]["frame"].is_null(), "{v}");
        assert!(v["gpu_ms"]["passes"].is_null(), "{v}");
        let why = v["unavailable"]["gpu_ms.frame"].as_str().unwrap();
        assert!(why.contains("timestamp"), "{why}");
        // The CPU side and the counts are still there.
        assert_eq!(v["counts"]["draw_calls"], 7);
        // How the instance batches were drawn rides along, all counts.
        let batches = v["counts"]["instance_batches"].as_object().unwrap();
        assert_eq!(batches.len(), 14);
        assert!(batches.values().all(Value::is_u64), "{batches:?}");
        assert_eq!(v["cpu_ms"]["total"], 4.0);
        assert_nulls_explained(&v);
    }

    #[test]
    fn gpu_times_wait_for_the_first_timed_frame() {
        let v = inspect(drawn(true, 0), None);
        assert_eq!(v["gpu_timing"]["available"], true);
        assert!(v["gpu_ms"]["frame"].is_null() && v["gpu_ms"]["passes"].is_null(), "{v}");
        assert_eq!(v["unavailable"]["gpu_ms.frame"], NOT_TIMED_YET);
        assert_nulls_explained(&v);
    }

    #[test]
    fn headless_and_undrawn_games_report_why_instead_of_zeros() {
        for (source, word) in [(Source::Headless, "headless"), (Source::NotDrawn, "drawn")] {
            let v = inspect(source, None);
            for group in ["cpu_ms", "gpu_ms", "counts"] {
                assert!(v[group].is_null(), "{group}: {v}");
                assert!(v["unavailable"][group].as_str().unwrap().contains(word), "{v}");
            }
            assert_eq!(v["gpu_timing"]["available"], false);
            assert_eq!(v["quality"]["level"], "high", "settings are known without frames");
            assert_nulls_explained(&v);
        }
    }

    #[test]
    fn missing_quality_settings_are_explained() {
        let v = render_inspector(&Inputs {
            source: Source::Headless,
            stats: RenderStats::default(),
            passes: &[],
            quality: None,
            pipelines: PipelineStats::default(),
            memory: None,
            adapter: None,
            history: &[],
            frustum_culled: Ok(0),
        });
        assert!(v["pipelines"].is_null(), "headless: {v}");
        assert!(v["memory_estimate_bytes"].is_null() && v["adapter"].is_null(), "{v}");
        assert_eq!(v["history"], json!([]));
        assert!(v["quality"].is_null());
        assert_nulls_explained(&v);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn quality_names_its_preset() {
        let q = quality_json(&crate::render::GraphicsQuality::medium());
        assert_eq!(q["level"], "medium");
        assert_eq!(q["anti_aliasing"], "fxaa");
        let q = quality_json(&crate::render::GraphicsQuality {
            shadow_cascades: 1,
            ..crate::render::GraphicsQuality::high()
        });
        assert_eq!(q["level"], "custom");
        assert_eq!(q["shadow_cascades"], 1);
    }
}

#[cfg(test)]
mod state_change_tests {
    use super::*;
    #[test]
    fn state_counters_are_additive_and_unavailable_without_rendering() {
        let mut input = Inputs {
            source: Source::Drawn(RenderDetail {
                state_changes: Some([2, 7, 3, 2, 11]),
                ..Default::default()
            }),
            stats: RenderStats::default(),
            passes: &[],
            quality: None,
            pipelines: PipelineStats::default(),
            memory: None,
            adapter: None,
            history: &[],
            frustum_culled: Ok(0),
        };
        let report = render_inspector(&input);
        assert_eq!(report["schema"], 1);
        assert_eq!(
            report["counts"]["state_changes"],
            crate::render::state_changes_json(Some([2, 7, 3, 2, 11]))
        );
        input.source = Source::Headless;
        let report = render_inspector(&input);
        assert!(report["counts"].is_null());
        assert!(report["unavailable"]["counts"].is_string());
        input.source = Source::Drawn(RenderDetail::default());
        let report = render_inspector(&input);
        assert!(report["counts"]["state_changes"].is_null());
        assert!(report["unavailable"]["counts.state_changes"].is_string());
    }
}
