struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

 struct VertexInput {
    @location(0) position: vec3<f32>,  
    @location(1) tex_coords: vec2<f32>,
 };

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_position: vec4<f32>,
};

const PI: f32 = 3.14159265359;
const ROWS: f32 = 100.0;   // Latitude divisions
const COLUMNS: f32 = 100.0;   // Longitude divisions
const RADIUS: f32 = 3.0;    // Sphere radius


@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;

    let quad_index = index / 6u;
    let vertex_in_quad = index % 6u;

    let row = quad_index / u32(COLUMNS);
    let col = quad_index % u32(COLUMNS);

    var v_offsets: array<f32, 6>;
    var u_offsets: array<f32, 6>;

    v_offsets = array<f32, 6>(0.0, 1.0, 0.0, 1.0, 1.0, 0.0);
    u_offsets = array<f32, 6>(0.0, 0.0, 1.0, 0.0, 1.0, 1.0);

    let v = (f32(row) / ROWS) + v_offsets[vertex_in_quad] / ROWS;
    let u = (f32(col) / COLUMNS) + u_offsets[vertex_in_quad] / COLUMNS;

    let phi = PI * (v - 0.5);
    let theta = 2.0 * PI * u;

    let x = RADIUS * cos(phi) * cos(theta);
    let y = RADIUS * cos(phi) * sin(theta);
    let z = RADIUS * sin(phi);

    let world_position = vec4<f32>(x, y, z, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.world_position = world_position;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.world_position;
}
