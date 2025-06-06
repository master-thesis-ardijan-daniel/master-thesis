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

const PI: f32 = 3.14159265358979323846264338327950288;

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

struct Metadata {
    tiles: array<TileMetadata, 256>,
}

struct TileMetadata {
    nw_lat: f32,
    nw_lon: f32,
    se_lat: f32,
    se_lon: f32,
    width: u32,
    height: u32,
    level: u32,
    pad_2: u32,
}

@group(1) @binding(0) var t_diffuse: texture_2d_array<f32>; 
@group(1) @binding(1) var s_diffuse: sampler;
@group(1) @binding(2) var<uniform> metadata: Metadata;


@fragment
fn fs_tiles(in: VertexOutput) -> @location(0) vec4<f32> {

    let pos = normalize(in.pos);
    let lon = (atan2(-pos.x, pos.y) / (2.0 * PI)) +0.5;
    let lat = (asin(-pos.z) / PI)+0.5 ;

    var highest_z = u32(0);
    var found_sample = false;
    var sample = vec4<f32>(); 

    for (var layer = 0; layer < 256 ; layer++){
        let metadata = metadata.tiles[layer];

        let nw_lat = (metadata.nw_lat + 90.0)  / 180.0;
        let nw_lon = (metadata.nw_lon + 180.0) / 360.0;
        let se_lat = (metadata.se_lat + 90.0)  / 180.0;
        let se_lon = (metadata.se_lon + 180.0) / 360.0;

        if (lat > nw_lat || lat < se_lat || lon < nw_lon || lon > se_lon) {
            continue;
        }

        let u = (lon - nw_lon) / (se_lon - nw_lon);
        let v = 1.-(lat - se_lat) / (nw_lat - se_lat);


        let scaled_u = u * f32(metadata.width)/256.;
        let scaled_v = v * f32(metadata.height)/256.;

        if (highest_z <= metadata.level) {
            found_sample = true;
            highest_z = metadata.level;
            sample = textureSample(
                t_diffuse,
                s_diffuse,
                vec2<f32>(scaled_u,scaled_v ),
                layer
            );
        };
    }

    if found_sample{
        return sample;
    }

    return vec4<f32>(0., lon, lat, 1.0);
}

// @fragment
// fn fs_tiles(in: VertexOutput) -> @location(0) vec4<f32> {
//     const PI: f32 = 3.14159265358979323846264338327950288;

//     let pos = normalize(in.pos);
//     let u = atan2(pos.x, -pos.y) / (2. * PI) + 0.5;
//     let v = asin(pos.z)/PI  + 0.5;

//     return vec4<f32>(u, v, 0.0, 1.0); // Debug color based on lon/lat
    // return textureSample(t_diffuse, s_diffuse, vec2<f32>(u, v), 0);
    // return vec4<f32>(in.pos, 1.0);
// }

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.pos, 1.0);
}

