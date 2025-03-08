use std::{collections::HashMap, f32::consts::PI};

use crate::types::Point;
use std::hash::{Hash, Hasher};
use wgpu::core::pipeline::ResolvedVertexState;
use winit::platform::wayland::EventLoopWindowTargetExtWayland;

pub const PHI: f32 = 1.618033988749894848204586834365638118_f32;

type Vertex = Point;
type Edge = [Vertex; 2];

// This function assumes the original vertecies are placed where they need to be.
// It will only apply the transformation function on the new vertecies.
// Vertex order matters, this algorithm is intended for
// counter clockwise ordered face vertex definition.
pub fn subdivide_icosphere<F>(
    vertecies: &[Vertex],
    faces: &[[usize; 3]],
    mut transformation_function: F,
) -> (Vec<Vertex>, Vec<[usize; 3]>)
where
    F: FnMut(&mut Vertex),
{
    // Transformation may be expensive and thus we want to avoid it if we can.
    // This cache only works as long as we use the even vertecies as keys
    // otherwise we can get rounding errors, thus its important not to mutate them
    // or change the algorithm in such a way that they change.
    let mut edges_with_odd_verts_cache: HashMap<[Vertex; 2], usize> = HashMap::new();

    let sort_edge = |a: Vertex, b: Vertex| -> [Vertex; 2] {
        if a.coordinates < b.coordinates {
            return [a, b];
        }
        [b, a]
    };

    let mut new_vertecies = vertecies.to_vec();

    // Creates a new vertex, moves the vertex and adds it to cache
    let mut create_new_vertex_on_edge_center = |a: Vertex, b: Vertex| -> usize {
        // Edges are independent of vertex order,
        // thus we sort the vertecies in order to use it as key
        let key = sort_edge(a, b);
        if let Some(i) = edges_with_odd_verts_cache.get(&key) {
            return *i;
        }

        let mut new_vert = (b - a) / 2. + a;
        // Move the vertex in 3d space
        transformation_function(&mut new_vert);

        let new_vertex_index = new_vertecies.len();
        new_vertecies.push(new_vert);

        edges_with_odd_verts_cache.insert(key, new_vertex_index);
        new_vertex_index
    };

    let mut new_faces = vec![];

    for i in 0..faces.len() {
        let face = faces[i];
        // Original vertecies
        let even_1 = face[0];
        let even_2 = face[1];
        let even_3 = face[2];

        // New vertecies
        let odd_1 = create_new_vertex_on_edge_center(vertecies[even_1], vertecies[even_2]);
        let odd_2 = create_new_vertex_on_edge_center(vertecies[even_2], vertecies[even_3]);
        let odd_3 = create_new_vertex_on_edge_center(vertecies[even_3], vertecies[even_1]);

        new_faces.push([even_1, odd_1, odd_3]);
        new_faces.push([even_2, odd_2, odd_1]);
        new_faces.push([even_3, odd_3, odd_2]);
        new_faces.push([odd_1, odd_2, odd_3]);
    }

    (new_vertecies, new_faces)
}

pub fn make_rotated_icosahedron() -> ([[f32; 3]; 12], [[usize; 3]; 20]) {
    let d = (1. + PHI.powi(2)).sqrt();
    let pentagram_height = 1.0 / d;
    let pentagram_radius = PHI / d;

    let mut vertecies: [[f32; 3]; 12] = [[0., 0., 0.]; 12];
    let mut faces: [[usize; 3]; 20] = [[0, 0, 0]; 20];

    vertecies[0] = [0., 0., 1.]; // Top vertex

    // Calculate the next 5 vertecies position
    for i in 0..5 {
        let angle = 2. * PI * i as f32 / 5.;
        let x = pentagram_radius * angle.cos();
        let y = pentagram_radius * angle.sin();
        let z = pentagram_height;
        vertecies[i + 1] = [x, y, z];
    }

    faces[0] = [0, 2, 1];
    faces[1] = [0, 3, 2];
    faces[2] = [0, 4, 3];
    faces[3] = [0, 5, 4];
    faces[4] = [0, 1, 5];

    //The bottom set of vertecies are a mirror along xy axis of the top.
    for v in 1..6 {
        let x = vertecies[v][0];
        let y = vertecies[v][1];
        let z = -pentagram_height;
        vertecies[v + 5] = [x, y, z];
    }

    //Fill in the central faces
    faces[5] = [1, 2, 7];
    faces[6] = [1, 7, 6];

    faces[7] = [2, 3, 8];
    faces[8] = [2, 8, 7];

    faces[9] = [3, 4, 9];
    faces[10] = [3, 9, 8];

    faces[11] = [4, 5, 10];
    faces[12] = [4, 10, 9];

    faces[13] = [5, 1, 6];
    faces[14] = [5, 6, 10];

    //Add bottom vertex and fill in the rest of the faces.
    vertecies[11] = [0., 0., -1.];

    faces[15] = [5 + 1, 5 + 2, 11];
    faces[16] = [5 + 2, 5 + 3, 11];
    faces[17] = [5 + 3, 5 + 4, 11];
    faces[18] = [5 + 4, 5 + 5, 11];
    faces[19] = [5 + 5, 5 + 1, 11];

    (vertecies, faces)
}
