use std::{collections::HashMap, f32::consts::PI};

use crate::types::Point;

pub const PHI: f32 = 1.618_034_f32;

type Vertex = Point;

#[derive(Debug)]
pub struct Icosphere {
    pub center: Point,
    pub radius: f32,
    pub vertecies: Vec<Point>, // Holds the vertecies for each subdivion level
    pub faces: Vec<Vec<[u32; 3]>>, //Holds the face indexes for each subdivison level
    vertex_subdiv_index: Vec<usize>,
    max_subdiv_level: usize,
    min_subdiv_level: usize,
    transformation_function: fn(Vertex) -> Vertex,
}

impl Icosphere {
    pub fn new(
        radius: f32,
        center: Point,
        max_subdiv_level: usize,
        min_subdiv_level: usize,
        vertex_transformation_function: fn(Vertex) -> Vertex,
    ) -> Self {
        let d = (1. + PHI.powi(2)).sqrt();
        let pentagram_height = 1.0 / d;
        let pentagram_radius = PHI / d;

        let mut vertecies: [Vertex; 12] = [[0., 0., 0.].into(); 12];
        let mut faces: [[u32; 3]; 20] = [[0, 0, 0]; 20];
        // Top vertex
        vertecies[0] = vertex_transformation_function(Into::<Point>::into([0., 0., 1.])); // Top vertex

        // Calculate the next 5 vertecies position
        let base_angle = 2. * PI * 1. / 5.;
        #[allow(clippy::needless_range_loop)]
        for i in 1..=10 {
            let mut angle = 2. * PI * i as f32 / 5.;
            let mut height = pentagram_height;
            if i > 5 {
                angle -= base_angle / 2.;
                height = -pentagram_height;
            }

            let x = pentagram_radius * angle.cos();
            let y = pentagram_radius * angle.sin();
            let z = height;
            vertecies[i] = vertex_transformation_function(Into::<Point>::into([x, y, z]));
        }

        // Bottom vertex
        vertecies[11] = vertex_transformation_function(Into::<Point>::into([0., 0., -1.])); // Top vertex

        faces[0] = [0, 2, 1];
        faces[1] = [0, 3, 2];
        faces[2] = [0, 4, 3];
        faces[3] = [0, 5, 4];
        faces[4] = [0, 1, 5];

        // Fill in the central faces
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

        faces[15] = [5 + 1, 5 + 2, 11];
        faces[16] = [5 + 2, 5 + 3, 11];
        faces[17] = [5 + 3, 5 + 4, 11];
        faces[18] = [5 + 4, 5 + 5, 11];
        faces[19] = [5 + 5, 5 + 1, 11];

        let mut new_icosphere = Self {
            center,
            radius,
            vertecies: vertecies.to_vec(),
            vertex_subdiv_index: vec![vertecies.len()],
            faces: vec![faces.to_vec()],
            max_subdiv_level,
            min_subdiv_level,
            transformation_function: vertex_transformation_function,
        };

        new_icosphere.subdivide_if_nessairy_and_clamp_level(min_subdiv_level);

        new_icosphere
    }

    fn subdivide_if_nessairy_and_clamp_level(&mut self, mut level: usize) -> usize {
        if level > self.max_subdiv_level {
            level = self.max_subdiv_level;
        }

        if level < self.min_subdiv_level {
            level = self.min_subdiv_level;
        }

        for _ in (self.faces.len())..=level {
            self.subdivide_icosphere();
        }

        level
    }

    pub fn get_subdivison_level_vertecies_and_faces(
        &mut self,
        mut level: usize,
    ) -> (&[Vertex], &[[u32; 3]]) {
        level = self.subdivide_if_nessairy_and_clamp_level(level);
        (
            &self.vertecies[..self.vertex_subdiv_index[level]],
            &self.faces[level],
        )
    }

    // // Used for debug
    pub fn get_subdivison_level_vertecies_and_lines(
        &mut self,
        mut level: usize,
    ) -> (&[Vertex], Vec<[u32; 2]>) {
        level = self.subdivide_if_nessairy_and_clamp_level(level);

        let mut lines = vec![];
        for face in self.faces[level].clone() {
            lines.push([face[0], face[1]]);
            lines.push([face[1], face[2]]);
            lines.push([face[2], face[0]]);
        }

        (&self.vertecies[..self.vertex_subdiv_index[level]], lines)
    }
    // This function assumes the original vertecies are placed where they need to be.
    // It will only apply the transformation function on the new vertecies.
    // Vertex order matters, this algorithm is intended for
    // counter clockwise ordered face vertex definition.
    fn subdivide_icosphere(&mut self) {
        // Transformation may be expensive and thus we want to avoid it if we can.
        // This cache only works as long as we use the even vertecies as keys
        // otherwise we can get rounding errors, thus its important not to mutate them
        // or change the algorithm in such a way that they change.
        let mut edges_with_odd_verts_cache: HashMap<[Vertex; 2], u32> = HashMap::new();

        let sort_edge = |a: Vertex, b: Vertex| -> [Vertex; 2] {
            if a.to_array() < b.to_array() {
                return [a, b];
            }
            [b, a]
        };
        // let mut new_vertecies = vec![];

        // Creates a new vertex, moves the vertex and adds it to cache
        let mut create_new_vertex_on_edge_center = |i_a: u32, i_b: u32| -> u32 {
            // Edges are independent of vertex order,
            // thus we sort the vertecies in order to use it as key
            let a = self.vertecies[i_a as usize];
            let b = self.vertecies[i_b as usize];

            let key = sort_edge(a, b);
            if let Some(i) = edges_with_odd_verts_cache.get(&key) {
                return *i;
            }

            let new_vert = (self.transformation_function)((b - a) / 2. + a);

            let new_vertex_index = self.vertecies.len() as u32;
            self.vertecies.push(new_vert);

            edges_with_odd_verts_cache.insert(key, new_vertex_index);
            new_vertex_index
        };

        let mut new_faces = vec![];

        for i in 0..self.faces[self.faces.len() - 1].len() {
            let face = self.faces[self.faces.len() - 1][i];
            // Original vertecies
            let even_1 = face[0];
            let even_2 = face[1];
            let even_3 = face[2];

            // New vertecies
            let odd_1 = create_new_vertex_on_edge_center(even_1, even_2);
            let odd_2 = create_new_vertex_on_edge_center(even_2, even_3);
            let odd_3 = create_new_vertex_on_edge_center(even_3, even_1);

            new_faces.push([even_1, odd_1, odd_3]);
            new_faces.push([even_2, odd_2, odd_1]);
            new_faces.push([even_3, odd_3, odd_2]);
            new_faces.push([odd_1, odd_2, odd_3]);
        }

        self.vertex_subdiv_index.push(self.vertecies.len());
        self.faces.push(new_faces);
    }
}
