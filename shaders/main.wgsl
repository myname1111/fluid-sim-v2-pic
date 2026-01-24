struct ScreenUniform {
    size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> screen_uniform: ScreenUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let pos = model.position / screen_uniform.size * 2.0 - 1.0;
    out.clip_position = vec4<f32>(pos.x, -pos.y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if length(in.clip_position.xy) < 200.0 {
        return vec4<f32>(0.3, 0.2, 0.1, 1.0);
    } else {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
}
