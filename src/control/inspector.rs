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

use crate::render::{RenderDetail, RenderStats};
use crate::{GpuPass, PipelineStats};

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
}

const NO_TIMESTAMPS: &str = "the GPU adapter doesn't support timestamp queries, so GPU time can't be measured";
const NOT_TIMED_YET: &str = "no frame's GPU timestamps have come back yet";
const MEMORY: &str = "GPU memory isn't estimated by the renderer yet";
const FRUSTUM: &str =
    "models skipped by frustum culling aren't counted yet; see counts.occluded_models for occlusion culling";

pub(super) fn render_inspector(inputs: &Inputs<'_>) -> Value {
    let mut unavailable = Map::new();
    let mut gone = |path: &str, why: &str| {
        unavailable.insert(path.into(), json!(why));
    };
    gone("memory_estimate_bytes", MEMORY);

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
            gone("counts.frustum_culled_models", FRUSTUM);
            let counts = json!({
                "draw_calls": s.draw_calls,
                "sprites": s.sprites,
                "models": s.instances,
                "triangles": s.triangles,
                "shadow_models": s.shadow_instances,
                "shadow_triangles": s.shadow_triangles,
                "occluded_models": s.occluded,
                "frustum_culled_models": null,
                "pipelines_built": s.pipelines_built,
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
        },
        "gpu_timing": gpu_timing,
        "cpu_ms": cpu_ms,
        "gpu_ms": gpu_ms,
        "counts": counts,
        "upload_bytes": upload_bytes,
        "memory_estimate_bytes": null,
        "pipelines": pipelines,
        "quality": quality,
        "unavailable": unavailable,
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
pub(super) fn quality_json(q: &crate::GraphicsQuality) -> Value {
    let level = match q.level() {
        crate::QualityLevel::Low => "low",
        crate::QualityLevel::Medium => "medium",
        crate::QualityLevel::High => "high",
        crate::QualityLevel::Ultra => "ultra",
        crate::QualityLevel::Custom => "custom",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(gpu_timestamps: bool, gpu_frames_timed: u64) -> Source {
        Source::Drawn(RenderDetail {
            gpu_timestamps,
            gpu_frames_timed,
            prepare_ms: 1.5,
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
        })
    }

    /// Every null field has a reason, and every reason names a null field.
    fn assert_nulls_explained(v: &Value) {
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
            "pipelines",
            "quality",
            "unavailable",
        ] {
            assert!(v.get(key).is_some(), "{key} missing from {v}");
        }
        assert!(v["upload_bytes"].is_u64(), "a drawn frame counts its uploads: {v}");
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
        });
        assert!(v["pipelines"].is_null(), "headless: {v}");
        assert!(v["quality"].is_null());
        assert_nulls_explained(&v);
    }

    #[cfg(feature = "3d")]
    #[test]
    fn quality_names_its_preset() {
        let q = quality_json(&crate::GraphicsQuality::medium());
        assert_eq!(q["level"], "medium");
        assert_eq!(q["anti_aliasing"], "fxaa");
        let q = quality_json(&crate::GraphicsQuality {
            shadow_cascades: 1,
            ..crate::GraphicsQuality::high()
        });
        assert_eq!(q["level"], "custom");
        assert_eq!(q["shadow_cascades"], 1);
    }
}
