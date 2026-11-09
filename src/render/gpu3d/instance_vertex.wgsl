fn static_instance(v: VertexIn) -> VertexIn {
    if v.m0.w != -2.0 { return v; }
    let index = static_indices[1u+v.instance_index];
    let base = 1u+index*6u;
    var out = v;
    out.m0 = items[base]; out.m1=items[base+1u]; out.m2=items[base+2u]; out.m3=items[base+3u]; out.color=items[base+4u];
    return out;
}
