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

    let stack_index = index / u32(SLICES); // Stack index (latitude)
    let slice_index = index % u32(SLICES); // Slice index (longitude)

    let v = f32(stack_index) / STACKS;
    let u = f32(slice_index) / SLICES;

    let phi = PI/2. - PI*f32(stack_index)/f32(STACKS);           // Latitude angle
    let theta = 2.0 * PI*f32(slice_index)/f32(SLICES);   // Longitude angle

    let x = RADIUS * cos(phi) * cos(theta);
    let y = RADIUS * cos(phi) * sin(theta);
    let z = RADIUS * sin(phi);

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
