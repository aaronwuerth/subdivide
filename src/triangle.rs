use nalgebra::Vector3;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
    ops::{Index, IndexMut},
};

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vector3<f64>>,
    pub faces: Vec<Face>,
    edges: HashMap<Edge, Vec<usize>>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vector3<f64>>, faces: Vec<(usize, usize, usize)>) -> Mesh {
        let faces: Vec<_> = faces.iter().map(|face| Face::new(*face)).collect();
        let edges = faces
            .iter()
            .enumerate()
            .flat_map(|(i, face)| {
                [
                    (Edge::new(face[0], face[1]), i),
                    (Edge::new(face[1], face[2]), i),
                    (Edge::new(face[2], face[0]), i),
                ]
            })
            .fold(
                HashMap::new(),
                |mut map: HashMap<Edge, Vec<usize>>, (edge, face_idx)| {
                    map.entry(edge).or_default().push(face_idx);
                    map
                },
            );

        Mesh {
            vertices,
            faces,
            edges,
        }
    }

    fn split_edge(&mut self, edge: Edge) -> Vec<Edge> {
        assert_ne!(edge.0, edge.1);

        let new_vertex = 0.5 * self.vertices[edge.0] + 0.5 * self.vertices[edge.1];
        let new_vertex_idx = self.vertices.len();
        self.vertices.push(new_vertex);

        let mut new_edges = vec![
            Edge::new(edge.0, new_vertex_idx),
            Edge::new(new_vertex_idx, edge.1),
        ];
        let mut edge_to_faces = vec![Vec::new(), Vec::new()];
        let mut third_edges = HashSet::new();

        let face_idxs = self.edges.remove(&edge).unwrap();

        for face_idx in face_idxs {
            let face = self.faces[face_idx];

            let (a, _) = face
                .vertex_indices
                .iter()
                .enumerate()
                .find(|(_, v)| **v == edge.0)
                .unwrap();

            let b_ccw = (a + 1) % 3;
            let b_cw = (a + 2) % 3;

            let b = if face[b_ccw] == edge.1 { b_ccw } else { b_cw };
            let c = if b == b_ccw { b_cw } else { b_ccw };

            let third_edge = Edge::new(face[c], new_vertex_idx);

            assert_ne!(edge, third_edge);

            let n = self.faces.len();
            self.faces.push(self.faces[face_idx]);

            // triangle a, c, d keeps a, c and replaces the original triangle
            // triangle b, c, d keeps b, c and is a new triangle
            let transfered_edge = Edge::new(face[b], face[c]);

            // remove old triangle from b, c and add new triangle to b, c
            let idx = self.edges[&transfered_edge]
                .iter()
                .position(|i| *i == face_idx)
                .unwrap();
            self.edges.get_mut(&transfered_edge).unwrap()[idx] = n;

            let face = self.faces[face_idx].vertex_indices;

            let split_face = |a: usize, b: usize| {
                let (a, b) = (a.min(b), a.max(b));
                let vertices = (face[a], face[b], new_vertex_idx);

                if b - a == 1 {
                    Face::new(vertices)
                } else {
                    Face::new((vertices.0, vertices.2, vertices.1))
                }
            };

            self.faces[face_idx] = split_face(a, c);
            self.faces[n] = split_face(b, c);

            // the old triangle always keeps the first vertex of a split edge
            edge_to_faces[0].push(face_idx);
            edge_to_faces[1].push(n);

            // add both to new common edge
            self.edges.entry(third_edge).or_default().push(face_idx);
            self.edges.entry(third_edge).or_default().push(n);
            third_edges.insert(third_edge);
        }

        new_edges
            .iter()
            .zip(edge_to_faces.drain(0..))
            .for_each(|(edge, faces)| {
                self.edges.insert(*edge, faces);
            });

        new_edges.extend(third_edges.iter());

        new_edges
    }

    pub fn split_edges_until(&mut self, length: f64) {
        #[derive(Debug)]
        struct QueueItem(f64, Edge);

        impl PartialEq for QueueItem {
            fn eq(&self, other: &Self) -> bool {
                self.0.eq(&other.0)
            }
        }

        impl Eq for QueueItem {}

        impl PartialOrd for QueueItem {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for QueueItem {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0
                    .partial_cmp(&other.0)
                    .unwrap()
                    .then_with(|| self.1.0.cmp(&other.1.0))
                    .then_with(|| self.1.1.cmp(&other.1.1))
            }
        }

        let mut queue: BinaryHeap<QueueItem> = BinaryHeap::new();

        let length_sq = length.powi(2);

        queue.extend(self.edges.keys().filter_map(|edge| {
            let edge_length_sq =
                (self.vertices[edge.1] - self.vertices[edge.0]).magnitude_squared();

            if edge_length_sq > length_sq {
                Some(QueueItem(edge_length_sq, *edge))
            } else {
                None
            }
        }));

        while let Some(QueueItem(edge_length_sq, edge)) = queue.pop() {
            if edge_length_sq <= length_sq {
                break;
            }

            let new_edges = self.split_edge(edge);

            queue.extend(new_edges.iter().filter_map(|new_edge| {
                let edge_length_sq =
                    (self.vertices[new_edge.1] - self.vertices[new_edge.0]).magnitude_squared();

                if edge_length_sq > length_sq {
                    Some(QueueItem(edge_length_sq, *new_edge))
                } else {
                    None
                }
            }))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Face {
    pub vertex_indices: [usize; 3],
}

impl Face {
    fn new(vertex_indices: (usize, usize, usize)) -> Face {
        let (a, b, c) = vertex_indices;

        Face {
            vertex_indices: [a, b, c],
        }
    }
}

impl Index<usize> for Face {
    type Output = usize;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.vertex_indices[idx]
    }
}

impl IndexMut<usize> for Face {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.vertex_indices[idx]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Edge(usize, usize);

impl Edge {
    fn new(a: usize, b: usize) -> Edge {
        Edge(a.min(b), a.max(b))
    }
}
