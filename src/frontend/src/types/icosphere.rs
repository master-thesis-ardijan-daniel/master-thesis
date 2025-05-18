use std::{borrow::Cow, collections::HashMap};

use super::HashablePoint;

pub const PHI: f32 = 1.618_034_f32;

type Point = glam::Vec3;
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
        let mut faces: [[u32; 3]; 20] = [[0, 0, 0]; 20];

        let vertecies: [Vertex; 12] = [
            vertex_transformation_function([-1., PHI, 0.].into()),
            vertex_transformation_function([1., PHI, 0.].into()),
            vertex_transformation_function([-1., -PHI, 0.].into()),
            vertex_transformation_function([1., -PHI, 0.].into()),
            vertex_transformation_function([0., -1., PHI].into()),
            vertex_transformation_function([0., 1., PHI].into()),
            vertex_transformation_function([0., -1., -PHI].into()),
            vertex_transformation_function([0., 1., -PHI].into()),
            vertex_transformation_function([PHI, 0., -1.].into()),
            vertex_transformation_function([PHI, 0., 1.].into()),
            vertex_transformation_function([-PHI, 0., -1.].into()),
            vertex_transformation_function([-PHI, 0., 1.].into()),
        ];

        faces[0] = [11, 0, 5];
        faces[1] = [5, 0, 1];
        faces[2] = [1, 0, 7];
        faces[3] = [7, 0, 10];
        faces[4] = [10, 0, 11];
        faces[5] = [5, 1, 9];
        faces[6] = [11, 5, 4];
        faces[7] = [10, 11, 2];
        faces[8] = [7, 10, 6];
        faces[9] = [1, 7, 8];
        faces[10] = [9, 3, 4];
        faces[11] = [4, 3, 2];
        faces[12] = [2, 3, 6];
        faces[13] = [6, 3, 8];
        faces[14] = [8, 3, 9];
        faces[15] = [9, 4, 5];
        faces[16] = [4, 2, 11];
        faces[17] = [2, 6, 10];
        faces[18] = [6, 8, 7];
        faces[19] = [8, 9, 1];

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
    ) -> (&[Vertex], Cow<'_, [u32]>) {
        level = self.subdivide_if_nessairy_and_clamp_level(level);
        (
            &self.vertecies[..self.vertex_subdiv_index[level]],
            Cow::Borrowed(self.faces[level].as_flattened()),
        )
    }

    // // Used for debug
    pub fn get_subdivison_level_vertecies_and_lines(
        &mut self,
        mut level: usize,
    ) -> (&[Vertex], Cow<'_, [u32]>) {
        level = self.subdivide_if_nessairy_and_clamp_level(level);

        let mut lines = vec![];
        for face in &self.faces[level] {
            lines.push([face[0], face[1]]);
            lines.push([face[1], face[2]]);
            lines.push([face[2], face[0]]);
        }

        (
            &self.vertecies[..self.vertex_subdiv_index[level]],
            Cow::Owned(lines.into_flattened()),
        )
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
        let mut edges_with_odd_verts_cache: HashMap<[HashablePoint; 2], u32> = HashMap::new();

        let sort_edge = |a: Vertex, b: Vertex| -> [HashablePoint; 2] {
            if a.to_array() < b.to_array() {
                return [a.into(), b.into()];
            }
            [b.into(), a.into()]
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
