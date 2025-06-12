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
@group(1) @binding(2) var t2_diffuse: texture_2d_array<f32>; 
@group(1) @binding(3) var s2_diffuse: sampler;
@group(1) @binding(4) var<uniform> metadata: Metadata;
@group(1) @binding(5) var<uniform> metadata_2: Metadata;


struct SampledTexture{
    highest_z: u32,
    sample: vec2<f32>,
    layer: u32,
    has_value: bool,
}

fn rgba_to_f32(rgba: vec4<f32>)->f32{
    let first = u32(rgba.r*255.);
    let second = u32(rgba.g*255.);
    let third = u32(rgba.b*255.);
    let fourth = u32(rgba.a*255.);

    let total:u32 = first | (second<<8)| (third<<16)| (fourth<<24);

    return bitcast<f32>(total);
}

fn sample_rgba(sample: SampledTexture)->vec4<f32>{
    return textureSample(
        t_diffuse,
        s_diffuse,
        sample.sample,
        sample.layer
    );
}

fn sample_2_f32(sample: SampledTexture)->f32{
    let rgba= textureSample(
        t2_diffuse,
        s2_diffuse,
        sample.sample,
        sample.layer
    );

    return rgba_to_f32(rgba);
}

fn tile_normalized(tile:TileMetadata)-> TileMetadata{
        let nw_lat = (tile.nw_lat + 90.0)  / 180.0;
        let nw_lon = (tile.nw_lon + 180.0) / 360.0;
        let se_lat = (tile.se_lat + 90.0)  / 180.0;
        let se_lon = (tile.se_lon + 180.0) / 360.0;

        return TileMetadata(
            nw_lat,
            nw_lon,
            se_lat,
            se_lon,
            tile.width,
            tile.height,
            tile.level,
            tile.data_type,
        );

}

fn intersects_with_tile(lat:f32, lon:f32, tile:TileMetadata)->bool{
    return !(lat > tile.nw_lat || lat < tile.se_lat || lon < tile.nw_lon || lon > tile.se_lon);
}

fn calc_uv(lat:f32, lon:f32, tile:TileMetadata)->vec2<f32>{
    let u = (lon - tile.nw_lon) / (tile.se_lon - tile.nw_lon);
    let v = 1.-(lat - tile.se_lat) / (tile.nw_lat - tile.se_lat);


    let scaled_u = u * f32(tile.width)/256.;
    let scaled_v = v * f32(tile.height)/256.;
    return vec2<f32>(scaled_u,scaled_v);
}


@fragment
fn fs_tiles(in: VertexOutput) -> @location(0) vec4<f32> {

    let pos = normalize(in.pos);
    let lon = (atan2(-pos.x, pos.y) / (2.0 * PI)) +0.5;
    let lat = (asin(-pos.z) / PI)+0.5 ;

    var samples: array<SampledTexture, 2> = array<SampledTexture, 2>(
        SampledTexture(0u, vec2<f32>(0.0), 0u, false),
        SampledTexture(0u, vec2<f32>(0.0), 0u, false),
    );   

    for (var layer:u32 = 0; layer < 256 ; layer++){
        let metadata = tile_normalized(metadata.tiles[layer]);
        let metadata_2 = tile_normalized(metadata_2.tiles[layer]);

        let tile_intersects = intersects_with_tile(lat,lon,metadata);
        let tile_2_intersects = intersects_with_tile(lat,lon,metadata_2);

        if tile_intersects{
            if (samples[0].highest_z<= metadata.level){
                samples[0].has_value = true;
                samples[0].sample = calc_uv(lat,lon,metadata);
                samples[0].layer = layer;
            }
        }

        if tile_2_intersects{
            if (samples[1].highest_z<= metadata_2.level){
                samples[1].has_value = true;
                samples[1].sample = calc_uv(lat,lon,metadata_2);
                samples[1].layer = layer;
            }
        }
    }

    if (!samples[1].has_value){
        discard;
    }


    // var return_color = sample_rgba(samples[0]);
    

    // if (samples[1].has_value){
    //     let pop_value = sample_f32(samples[1]);

    //     let pop_color = sample_gradient(pop_value,1000000.,1);

    //     return_color=return_color*0.01+pop_color;
    // }

    // if (samples[1].has_value){
        // let lp_value = sample_2_f32(samples[1]);

        // let lp_color = sample_gradient(lp_value,30.,2);

        // return_color=return_color*0.1+lp_value;
    let return_color= textureSample(
        t2_diffuse,
        s2_diffuse,
        samples[1].sample,
        samples[1].layer
    );

    // }

    return return_color;
}


fn sample_gradient(i: f32, max_value:f32, gradient_index: u32)-> vec4<f32>{

    const grad_1 = array<vec4<f32>, 4>(
        vec4<f32>(0., 0., 0.,0.),
        vec4<f32>(0., 0.4, 1.,1.), 
        vec4<f32>(0., 0.4, 1.,1.), 
        vec4<f32>(0., 0.4, 1.,1.) 
    );
    const grad_2 = array<vec4<f32>, 4>(
        vec4<f32>(0., 0., 0.,0.),
        vec4<f32>(0.7, 0.7, 0.2,1.), 
        vec4<f32>(1., 0.65, 0.3,1.), 
        vec4<f32>(1., 1.0, 1.,1.) 
    );

    var gradient: array<vec4<f32>, 4>;
    if (gradient_index == 1u) {
        gradient = grad_1;
    } else {
        gradient = grad_2;
    }



    const n_colors = 4.;

    let sample_location = (i/max_value)*(n_colors-1.);
    let index = u32(sample_location);
    let mix_val = fract(sample_location);

    let c1 = gradient[index];
    let c2 = gradient[index + 1];

    return mix(c1,c2,mix_val);
}






@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.pos, 1.0);
}

