struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
      @location(0) position: vec3<f32>,  
      @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) pos: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;

    let world_position = vec4<f32>(model.position, 1.0);
    out.pos = vec3<f32>(world_position.x,world_position.y,world_position.z);
    out.clip_position = camera.view_proj * world_position;

    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.pos, 1.0);
}
