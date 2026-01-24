struct ScreenUniform {
    size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> screen_uniform: ScreenUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct instanceInput {
    @location(1) position: vec2<f32>,
    @location(2) radius: f32
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) rel_pos: vec2<f32>
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    model: VertexInput,
    instance: instanceInput
) -> VertexOutput {
    var out: VertexOutput;
    let world_position = model.position * instance.radius + instance.position;
    let pos = world_position / screen_uniform.size * 2.0 - 1.0;
    out.clip_position = vec4<f32>(pos.x, -pos.y, 0.0, 1.0);
    out.rel_pos = model.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if length(in.rel_pos) < 1.0 {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    } else {
        return vec4<f32>(0.0);
    }
}
