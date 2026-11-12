// ANCHOR: material_unlit
fn surface(s: Surface) -> Surface {
    var out = s;
    out.unlit = true;
    out.alpha *= select(0.0, 1.0, s.uv.x >= 0.5);
    return out;
}
// ANCHOR_END: material_unlit
