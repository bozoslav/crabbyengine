### Added

- Opt-in scene quality autotuning using fresh GPU timestamp samples, bounded render-scale and sun-shadow steps, hysteresis, cooldowns, explicit-setting pins and deterministic capture/headless freezes.
- Rust quality policy/status APIs and additive `perf.get` render-inspector diagnostics with the last 16 decisions. Existing Rust and native APIs remain compatible.
