use std::f32::consts::PI;

pub const PHI: f32 = 1.618033988749894848204586834365638118_f32;

pub fn subdivide_icosphere(vertecies: &[[f32; 3]], faces: &[[usize; 3]]) {}

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
