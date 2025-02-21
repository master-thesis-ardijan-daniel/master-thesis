struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: Camera;

// struct VertexInput {
      // @location(0) position: vec3<f32>,  
      // @location(1) tex_coords: vec2<f32>,
// };


// struct VertexInput {
      // @builtin(position) position: vec3<f32>,  
      // @location(1) tex_coords: vec2<f32>,
// };

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

const PI: f32 = 3.14159265359;
const STACKS: f32 = 20.0;   // Latitude divisions
const SLICES: f32 = 40.0;   // Longitude divisions
const RADIUS: f32 = 2.0;    // Sphere radius

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let v_idx = index / u32(SLICES); // Stack index (latitude)
    let u_idx = index % u32(SLICES); // Slice index (longitude)

    let v = f32(v_idx) / STACKS;
    let u = f32(u_idx) / SLICES;

    let phi = v * PI;           // Latitude angle
    let theta = u * 2.0 * PI;   // Longitude angle

    let x = RADIUS * sin(phi) * cos(theta);
    let y = RADIUS * sin(theta);
    let z = RADIUS * cos(phi) * cos(theta);

    let world_position = vec4<f32>(x, y, z, 1.0);
    // Transform to clip space
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = vec2<f32>(u, v);

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    return vec4<f32>(in.clip_position.x,in.clip_position.y,in.clip_position.z, 1.0);
    // return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
