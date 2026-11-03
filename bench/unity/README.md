# The 3D benchmark scene in Unity

The scene from `examples/3d.rs`, rebuilt the way a normal Unity game is
written: one GameObject per cube, each with a `MeshFilter`, a
`MeshRenderer` and a `Spin` script (`Update()` turns it). It's used for the
"3D: Crabby vs Unity" table on the benchmarks page.

The two scenes match:

- N cubes 0.6 wide in a grid 1.6 apart, each turning around Y at its own
  speed, every frame;
- 12 spheres from smooth to rough, plastic and metal;
- a 400 x 400 ground;
- one sun from (-0.8, -0.6, -0.4) with soft shadows: one shadow map,
  2048², reaching 60 units;
- sky and ground ambient light;
- 4x MSAA, physically based materials;
- the same camera (60° field of view, the demo's starting orbit).

URP's extras that Crabby doesn't have are off: SSAO, post-processing and
HDR. Unity's 3D axes are left-handed and Crabby's are right-handed, so
Unity sees the field mirrored. The work is the same.

Other differences:

- The cubes share 64 tinted materials instead of one color each, which
  favors Unity: one material per cube would break its batching.
- The player is built with Mono, Unity's default script backend. IL2CPP
  would make the 100k `Update()` calls somewhat faster.

## Run it

1. Make a Unity 6 project from the "Universal 3D" (URP) template.
2. Copy `Assets/Bench` into it.
3. Build the macOS player:

   ```sh
   Unity -batchmode -quit -projectPath <project> -executeMethod BenchBuild.Build
   ```

4. Run it fullscreen at the display's resolution:

   ```sh
   Build/CrabbyBench3D.app/Contents/MacOS/CrabbyBench3D -cubes 100000 -seconds 10 \
     -out unity.json -screen-fullscreen 1 -window-mode borderless \
     -screen-width 3456 -screen-height 2234
   ```

The Crabby side:

```sh
CUBES=100000 BENCH_SECONDS=10 cargo run --release --features 3d --example 3d
```

Both run with vsync off. They warm up for 3 seconds, measure for the next
10, print the averages as JSON and quit. Unity's JSON includes its own
main-thread, render-thread and GPU times from `FrameTimingManager`.
