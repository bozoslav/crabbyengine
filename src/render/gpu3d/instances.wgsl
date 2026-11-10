struct Cell { lo:vec4<f32>, hi:vec4<f32> };
struct View { planes:array<vec4<f32>,6>, camera:vec4<f32>, occlusion:vec4<f32>, band:vec4<f32> };
@group(0) @binding(0) var<storage,read> data:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> indices:array<u32>;
@group(0) @binding(2) var<storage,read_write> args:array<atomic<u32>>;
@group(0) @binding(3) var<uniform> view:View;
@group(0) @binding(4) var<storage,read> depth:array<f32>;
fn hidden(center:vec3<f32>, radius:f32) -> bool {
    if view.occlusion.x == 0.0 || depth[3] == 0.0 {return false;}
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
fn visible(center:vec3<f32>, radius:f32) -> bool {
    if view.band.z != 0.0 && distance(center,view.camera.xyz)-radius > view.camera.w {return false;}
    for (var i=0u;i<6u;i++) {if dot(view.planes[i].xyz,center)+view.planes[i].w < -radius {return false;}}
    return !hidden(center,radius);
}
@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group:vec3<u32>, @builtin(local_invocation_index) lane:u32) {
    let cell_index=group.x+group.y*256u;
    if cell_index >= u32(data[0].y) {return;}
    let base=1u+u32(data[0].x)*6u+cell_index*2u;
    let cell=Cell(data[base],data[base+1u]);
    if view.band.z == 0.0 {
        // Cell bounds include every origin as well as the mesh spheres.
        // AABB distances therefore conservatively bound origin distances,
        // including off-centre meshes and proxies.
        let near=max(max(cell.lo.xyz-view.camera.xyz,view.camera.xyz-cell.hi.xyz),vec3<f32>(0.0));
        let far=max(abs(cell.lo.xyz-view.camera.xyz),abs(cell.hi.xyz-view.camera.xyz));
        if dot(near,near) >= view.band.y*view.band.y || dot(far,far) < view.band.x*view.band.x {return;}
    }
    if !visible((cell.lo.xyz+cell.hi.xyz)*0.5,length(cell.hi.xyz-cell.lo.xyz)*0.5) {return;}
    let start=bitcast<u32>(cell.lo.w); let end=start+bitcast<u32>(cell.hi.w);
    for(var i=start+lane;i<end;i+=64u) {
        if view.band.z == 0.0 {
            let d=distance(data[1u+i*6u+3u].xyz,view.camera.xyz);
            if d < view.band.x || d >= view.band.y {continue;}
        }
        let sphere=data[1u+i*6u+5u];
        if visible(sphere.xyz,sphere.w) {
            let slot=atomicAdd(&args[1],1u);
            indices[1u+slot]=i;
            for(var part=1u;part<arrayLength(&args)/5u;part++) {atomicAdd(&args[part*5u+1u],1u);}
        }
    }
}
