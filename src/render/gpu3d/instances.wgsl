// Culls the static batch views of one page in a single dispatch.
//
// `data` holds a pool's batches as vec4s: per batch a block of a header,
// six vec4s per instance and two per cell. `indices` is a page: its root,
// this frame's jobs, and the visible instance list of each of its views.
// `args` holds the views' indirect draw arguments (five words per mesh
// part). `depth` holds what all pages share this frame: the occlusion
// grid's header and depths, and the views (frustum planes and eye).
// A job culls one batch for one view and distance level; its cells take
// `cells` consecutive workgroups starting at `first`.
//
//   root (indices[0..8]): jobs, first job's index, workgroups, side, 0...
//   job (12 words): block, visibility, arguments, parts,
//                   first, cells, view, flags, near, far (as f32 bits), 0, 0
//   view (28 floats from depth[depth[20] as bits]): six planes, (eye, 0)
//
// `block` indexes `data`; `visibility` indexes `indices` (the list starts
// one past it); `arguments` indexes `args`.
// Flags: 1 = one mesh with a maximum distance to the instance's sphere,
// 2 = test against the depth grid.
@group(0) @binding(0) var<storage,read> data:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> indices:array<u32>;
@group(0) @binding(2) var<storage,read_write> args:array<atomic<u32>>;
@group(0) @binding(3) var<storage,read> depth:array<f32>;
fn hidden(center:vec3<f32>, radius:f32) -> bool {
    if depth[3] == 0.0 {return false;}
    let vp=mat4x4<f32>(vec4<f32>(depth[4],depth[5],depth[6],depth[7]),vec4<f32>(depth[8],depth[9],depth[10],depth[11]),vec4<f32>(depth[12],depth[13],depth[14],depth[15]),vec4<f32>(depth[16],depth[17],depth[18],depth[19]));
    let extent=vec3<f32>(radius+depth[2]);
    var lo=vec2<f32>(1e30); var hi=vec2<f32>(-1e30); var nearest=1e30;
    for(var i=0u;i<8u;i++) {
        let sign=vec3<f32>(select(-1.0,1.0,(i&1u)!=0u),select(-1.0,1.0,(i&2u)!=0u),select(-1.0,1.0,(i&4u)!=0u));
        let clip=vp*vec4<f32>(center+extent*sign,1.0);
        if clip.w <= 1e-5 {return false;}
        lo=min(lo,clip.xy/clip.w); hi=max(hi,clip.xy/clip.w); nearest=min(nearest,clip.z/clip.w);
    }
    if any(lo < vec2<f32>(-1.0)) || any(hi > vec2<f32>(1.0)) || nearest < 0.0 {return false;}
    let w=u32(depth[0]); let h=u32(depth[1]);
    let a=min(vec2<u32>((vec2<f32>(lo.x,-hi.y)*0.5+0.5)*vec2<f32>(f32(w),f32(h))),vec2<u32>(w-1u,h-1u));
    let b=min(vec2<u32>((vec2<f32>(hi.x,-lo.y)*0.5+0.5)*vec2<f32>(f32(w),f32(h))),vec2<u32>(w-1u,h-1u));
    for(var y=a.y;y<=b.y;y++) {for(var x=a.x;x<=b.x;x++) {if nearest <= depth[24u+y*w+x] {return false;}}}
    return true;
}
fn word(i:u32) -> vec4<f32> {return vec4<f32>(depth[i],depth[i+1u],depth[i+2u],depth[i+3u]);}
// `view`: the index in `depth` of the view's first plane; `far`: the
// maximum distance to the sphere (flag 1 only).
fn visible(center:vec3<f32>, radius:f32, view:u32, flags:u32, far:f32) -> bool {
    if (flags & 1u) != 0u && distance(center,word(view+24u).xyz)-radius > far {return false;}
    for (var i=0u;i<6u;i++) {let p=word(view+i*4u); if dot(p.xyz,center)+p.w < -radius {return false;}}
    return (flags & 2u) == 0u || !hidden(center,radius);
}
@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group:vec3<u32>, @builtin(local_invocation_index) lane:u32) {
    let jobs=indices[0]; let table=indices[1]; let side=indices[3];
    let slot=group.x+(group.y+group.z*side)*side;
    if slot >= indices[2] {return;}
    // The last job that starts at or before this workgroup.
    var lo=0u; var hi=jobs;
    while hi-lo > 1u {
        let mid=(lo+hi)/2u;
        if indices[table+mid*12u+4u] <= slot {lo=mid;} else {hi=mid;}
    }
    let job=table+lo*12u;
    let a=vec4<u32>(indices[job],indices[job+1u],indices[job+2u],indices[job+3u]);
    let b=vec4<u32>(indices[job+4u],indices[job+5u],indices[job+6u],indices[job+7u]);
    let band=vec2<f32>(bitcast<f32>(indices[job+8u]),bitcast<f32>(indices[job+9u]));
    let cell_index=slot-b.x;
    if cell_index >= b.y {return;}
    let view=bitcast<u32>(depth[20])+b.z*28u;
    let eye=word(view+24u).xyz;
    let legacy=(b.w & 1u) != 0u;
    let block=a.x;
    let base=block+1u+u32(data[block].x)*6u+cell_index*2u;
    let cell_lo=data[base]; let cell_hi=data[base+1u];
    if !legacy {
        // Cell bounds include every origin as well as the mesh spheres.
        // AABB distances therefore conservatively bound origin distances,
        // including off-centre meshes and proxies.
        let near=max(max(cell_lo.xyz-eye,eye-cell_hi.xyz),vec3<f32>(0.0));
        let far=max(abs(cell_lo.xyz-eye),abs(cell_hi.xyz-eye));
        if dot(near,near) >= band.y*band.y || dot(far,far) < band.x*band.x {return;}
    }
    if !visible((cell_lo.xyz+cell_hi.xyz)*0.5,length(cell_hi.xyz-cell_lo.xyz)*0.5,view,b.w,band.y) {return;}
    let start=bitcast<u32>(cell_lo.w); let end=start+bitcast<u32>(cell_hi.w);
    for(var i=start+lane;i<end;i+=64u) {
        if !legacy {
            let d=distance(data[block+1u+i*6u+3u].xyz,eye);
            if d < band.x || d >= band.y {continue;}
        }
        let sphere=data[block+1u+i*6u+5u];
        if visible(sphere.xyz,sphere.w,view,b.w,band.y) {
            let found=atomicAdd(&args[a.z+1u],1u);
            indices[a.y+1u+found]=i;
            for(var part=1u;part<a.w;part++) {atomicAdd(&args[a.z+part*5u+1u],1u);}
        }
    }
}
// Zeroes the instance counts of the frame's jobs, before `main` counts.
@compute @workgroup_size(64)
fn reset(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x >= indices[0] {return;}
    let job=indices[1]+id.x*12u;
    let arguments=indices[job+2u]; let parts=indices[job+3u];
    for(var part=0u;part<parts;part++) {atomicStore(&args[arguments+part*5u+1u],0u);}
}
