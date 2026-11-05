# Importing from Unity

`crabby_unity` reads a Unity project and turns its scenes and prefabs into
Crabby [scenes](scenes.md): every drawn mesh becomes a `.glb` with its
materials, its textures become PNGs, and the scene is a `crabby-world`
file of `Transform3D` and `Model` entities that `load_scene` loads. It
runs once, as a tool, and writes into your game's `assets/`.

```sh
cargo run --release -p crabby_unity --example import -- \
    MyUnityGame my_game/assets/town town/ Assets/Scenes/Town.unity
```

That writes `my_game/assets/town/Town.json` (load it with
`ctx.load_scene("town/Town.json")`), the meshes in `town/meshes/` and their
textures in `town/meshes/textures/`, then says what it left out and which
shaders the materials used. Give it several scenes or prefabs at once and
they share their meshes and textures; `ONE=all.json` puts all their
entities in one scene.

To look around what came out:

```sh
SCENE=town/Town.json cargo run --release --features 3d --example scene_viewer
```

Unity's +Z is Crabby's -Z: everything is mirrored along Z on the way, so a
scene looks the same, and positions and rotations come out in Crabby's
space.

## What comes over

- **Meshes** from mesh assets (`.asset`), FBX and OBJ files (scaled and
  oriented as Unity's model importer does), and Unity's built-in Cube,
  Plane and Quad. Each submesh keeps its material. A model imported with
  **Bake Axis Conversion** has its file's axes (Z-up, say) turned into
  Unity's in the mesh itself, as Unity does, so parts placed one by one
  in a prefab line up.
- **Materials** of URP's Lit, Simple Lit and Unlit shaders, and of other
  shaders that use the same property names: base color and map, normal
  map, metallic and smoothness (packed into glTF's occlusion, roughness
  and metalness image), emission, tiling, cutout, transparency and
  two-sided, plus occlusion. A custom shader is drawn as plain PBR by the
  same rules; the report lists the shaders used, so you know which need a
  Crabby [shader](3d.md) of their own.
- **Textures** from image files, at most as large as their import settings
  allow, and from texture assets saved uncompressed.
- **Prefabs** inside prefabs, with their overrides, baked into one flat
  list of entities placed in the world.
- **Colliders**, if you ask (`importer.colliders(true)`): box, sphere,
  capsule and mesh colliders become entities with a fixed `RigidBody3D`
  and a `Collider3D` ([3D physics](physics3d.md)), the object's scale
  worked into the shape. A mesh collider's mesh is written as a `.glb` and
  named in the scene by file; `ctx.load_scene` makes a triangle mesh of
  it (a convex hull for a convex one), once per mesh and scale, and saving
  the scene names the file again. Triggers are left out.

Vertex colors are kept only for shaders you name (see below): URP's
shaders ignore them, so most meshes are drawn as Unity draws them without.

Renderers keep their Cast Shadows setting: Off gets a `ShadowMesh` of
none, and Shadows Only (often a simplified stand-in for the shadows of the
pieces around it) becomes a `ShadowMesh` with no `Model`, drawn only into
the shadow maps.

Left out, and counted in the report: far levels of detail (only LOD 0 is
kept), inactive objects and disabled renderers, skinned meshes, the
built-in Sphere, Capsule and Cylinder, compressed meshes, and meshes or
textures it can't find or read.

## Your own shaders for imported materials

Materials of the project's own shaders come over as plain PBR. To draw
them with a Crabby [shader](3d.md) of your own, tell the importer which
shaders those are:

```rust
import.custom_shader("Assets/Shaders/Wall.shader"); // as the report lists it
```

Their meshes then keep their vertex colors and UV set 1 (which custom
shaders often read; in Crabby's shaders they're `s.color` and `s.uv1`,
UV set 1 unflipped, as Unity has it), and the importer writes
`materials.json` next to the scenes: each such material by name, with its
shader and every number, color and texture it sets, the textures written
as PNGs. In the game, `loaded_materials(name)` gives every material loaded
from glTF files under that name, with what it was made from, so you can
keep that and change the shader:

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/three.rs:loaded_materials}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:loaded_materials}}
```

</div>
</div>

Call it after the scene (or model) is loaded: models loaded later aren't
in it yet. C: `crabby_ctx_loaded_materials`.

A shader that reads something else per vertex, such as a second UV set,
can have it put in the vertex colors instead, which is what Crabby's
shaders get per vertex besides one UV set:

```rust
// A river shader with its flow in UV set 1: speed up to 4, foam 0 to 1.
import.vertex_colors("Assets/Shaders/River.shader", |mesh, v| {
    let [speed, foam] = mesh.uvs[1].get(v).copied().unwrap_or_default();
    [speed / 4.0, foam, 0.0, 1.0]
});
```

Each channel keeps 8 bits, from 0 to 1, so scale values into that range
and back in the shader.

## Terrain

Unity terrain (`TerrainData`, text or binary) reads as a
`UnityTerrain`: its heights in world units, already in Crabby's row order,
ready for a [terrain tile](3d.md), and the layers painted on it with
their weight maps.

```rust
let file = crabby_unity::UnityFile::read("MyUnityGame/Assets/Terrain/Hills.asset")?;
let data = file.of_class(156).next().expect("a TerrainData");
let hills = crabby_unity::UnityTerrain::from_object(data)?;
// In the game (or written to a file the game reads):
let tile = ctx.create_terrain_tile(&hills.heights, hills.size, ground);
let [x, y, z] = hills.corner([0.0, 0.0, 0.0]); // the Unity Terrain's position
ctx.world.spawn((Transform3D::at(vec3(x, y, z)), tile));
```

The weight maps are `Texture2D`s in the same file: read them with
`UnityTexture` and their rows already line up with the tile.

## From C and C++

The importer is a Rust tool: run it once, as above. What it writes are
ordinary scene, glTF and PNG files, which C and C++ games load with
`ctx.load_scene(path)` (C: `crabby_ctx_load_scene`) like any other.
