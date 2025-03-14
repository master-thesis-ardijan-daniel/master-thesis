struct Camera {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
      @location(0) position: vec3<f32>,  
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) pos: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let world_position = vec4<f32>(model.position, 1.0);
    out.pos = vec3<f32>(world_position.x, world_position.y, world_position.z);
    out.clip_position = camera.view_proj * world_position;

    return out;
}

@group(1) @binding(0) var t_diffuse: texture_2d<f32>; 
@group(1) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    const PI: f32 = 3.14159265358979323846264338327950288;

    let pos = normalize(in.pos);
    let u = atan2(pos.x, -pos.y) / (2. * PI) + 0.5;
    let v = asin(pos.z)/PI  + 0.5;

    return textureSample(t_diffuse, s_diffuse, vec2<f32>(u, v));
}

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.pos, 1.0);
}
