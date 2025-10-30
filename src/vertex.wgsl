struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec2<f32>,
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(vertex_index >> 1);
    let y = f32(vertex_index & 1);
    out.position = vec2(x, y) * 2 - 1;
    out.clip_position = vec4(out.position, 0, 1);
    return out;
}
