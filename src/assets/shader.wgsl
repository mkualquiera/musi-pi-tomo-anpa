struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
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
    out.uv = model.uv;
    return out;
}

struct EngineColor {
    color: vec4<f32>,
}

@group(1) @binding(1)
var<uniform> engine_color: EngineColor;

@group(2) @binding(2)
var gizmo_texture: texture_2d<f32>;
@group(2) @binding(3)
var gizmo_sampler: sampler;

struct SpriteSpec {
    use_texture_and_padding: vec4<u32>, // Use a vec4 to ensure alignment
    region_start_and_end: vec4<f32>, // Start and end of the sprite region
    tiles_info: vec4<u32>, // Number of tiles and selected tile
}

@group(3) @binding(4)
var<uniform> sprite_spec: SpriteSpec;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    if (1 == 1) { 
        let region_start = sprite_spec.region_start_and_end.xy;
        let region_end = sprite_spec.region_start_and_end.zw;
        let num_tiles = vec2<f32>(f32(sprite_spec.tiles_info.x), f32(sprite_spec.tiles_info.y));
        let selected_tile = vec2<f32>(f32(sprite_spec.tiles_info.z), f32(sprite_spec.tiles_info.w));
        let tile_size = (region_end - region_start)
            / num_tiles;
        let uv_offset = region_start + selected_tile * tile_size;

        //tex_color = textureSample(gizmo_texture, gizmo_sampler, uv_offset + in.uv * tile_size);
        tex_color = textureSample(gizmo_texture, gizmo_sampler, in.uv * tile_size + uv_offset);
    }
    return vec4<f32>(in.color, 1.0) * engine_color.color * tex_color;
}