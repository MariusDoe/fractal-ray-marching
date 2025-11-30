@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fragment_main(@location(0) screen_position: vec2<f32>) -> @location(0) vec4<f32> {
    let flipped_uv = (screen_position + 1) * 0.5;
    let uv = vec2(flipped_uv.x, 1 - flipped_uv.y);
    return textureSample(texture, texture_sampler, uv);
}
