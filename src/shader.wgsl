struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct Transform {
    matrix: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> transform: Transform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = transform.matrix * vec4<f32>(model.position, 1.0);
    return out;
}

struct EngineColor {
    color: vec4<f32>,
}

@group(1) @binding(1)
var<uniform> engine_color: EngineColor;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0) * engine_color.color;
}