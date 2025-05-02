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

struct TileMetadata {
    nw_lat: f32,
    nw_lon: f32,
    se_lat: f32,
    se_lon: f32,
    width: u32,
    height: u32,
    pad_1: u32,
    pad_2: u32,
}

@group(1) @binding(0) var t_diffuse: texture_2d_array<f32>; 
@group(1) @binding(1) var s_diffuse: sampler;
@group(1) @binding(2) var<storage,read> metadata: array<TileMetadata>;

@fragment
fn fs_tiles(in: VertexOutput) -> @location(0) vec4<f32> {
    const PI: f32 = 3.14159265358979323846264338327950288;

    let nw_lat = radians(metadata.nw_lat);
    let nw_lon = radians(metadata.nw_lon);
    let se_lat = radians(metadata.se_lat);
    let se_lon = radians(metadata.se_lon);

    let pos = normalize(in.pos);
    let lon = atan2(pos.x, -pos.y) / (2. * PI);
    let lat = asin(pos.z)/PI  + 0.;

    if (lat > nw_lat || lat < se_lat || lon < nw_lon || lon > se_lon) {
        discard;
    }

    let u = (lon - nw_lon) / (se_lon - nw_lon);
    let v = (lat - se_lat) / (nw_lat - se_lat);

    let layer =0;

    return textureSample(t_diffuse, s_diffuse, vec2<f32>(u, v),layer);
}

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.pos, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0., 0., 0., 0.);
}
