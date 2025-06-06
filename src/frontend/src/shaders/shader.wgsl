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
    data_type: u32,
}

@group(1) @binding(0) var t_diffuse: texture_2d_array<f32>; 
@group(1) @binding(1) var s_diffuse: sampler;
@group(1) @binding(2) var<uniform> metadata: Metadata;


@fragment
fn fs_tiles(in: VertexOutput) -> @location(0) vec4<f32> {

    let pos = normalize(in.pos);
    let lon = (atan2(-pos.x, pos.y) / (2.0 * PI)) +0.5;
    let lat = (asin(-pos.z) / PI)+0.5 ;

    var highest_z_color = u32(0);
    var highest_z_pop = u32(0);
    var found_sample_color = false;
    var found_sample_pop = false;
    var sample_texture = vec4<f32>(); 
    var sample_pop = vec2<f32>(); 
    var pop_layer = i32(0); 
    var sample_color = vec2<f32>(); 
    var color_layer = i32(0); 

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

        if metadata.data_type==0{
            if (highest_z_color <= metadata.level) {
                found_sample_color = true;
                highest_z_color = metadata.level;
                sample_color = vec2<f32>(scaled_u,scaled_v);
                color_layer=layer;
            };
        } else {
            if (highest_z_pop <= metadata.level) {
                found_sample_pop = true;
                highest_z_pop = metadata.level;
                sample_pop = vec2<f32>(scaled_u,scaled_v);
                pop_layer=layer;
            };
        }

    }

    if (!found_sample_pop && !found_sample_color){
        discard;
    }


    var return_color = vec4<f32>(0.2,0.2,0.2,1.0);

    if found_sample_color{
            return_color = textureSample(
                t_diffuse,
                s_diffuse,
                sample_color,
                color_layer
            );
        }

    if found_sample_pop{
            let sample_rgba = textureSample(
                t_diffuse,
                s_diffuse,
                sample_pop,
                pop_layer
            );

            let first = u32(sample_rgba.r*255.);
            let second = u32(sample_rgba.g*255.);
            let third = u32(sample_rgba.b*255.);
            let fourth = u32(sample_rgba.a*255.);

            let total:u32 = first | (second<<8)| (third<<16)| (fourth<<24);

            // let v: u32 = pack4x8unorm(sample_rgba);

            // let v : f32 = textureLoad(t_diffuse, vec2<i32>(x, y), layer, mip = 0).r;

            var population_value = bitcast<f32>(total);
            if population_value>0.{

                population_value = 1.+population_value/1000.;
                return_color= return_color*population_value;
            }
            // let population_value = 1.;

    }


    return return_color;
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

