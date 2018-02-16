use std::cmp::max;

pub trait Connection {
    fn has_inputs(&self) -> bool {
        true
    }
    fn has_outputs(&self) -> bool {
        false
    }
    fn available_inputs(&self) -> usize {
        1
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Vertex<T> {
    pub data: T,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Edges<T> {
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub data: T,
}

pub struct Graph<V, E> {
    pub vertices: Vec<Vertex<V>>,
    pub edges: Vec<Edges<E>>,
}

impl<V, E> Graph<V, E> {
    pub fn new() -> Graph<V, E> {
        Graph {
            vertices: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn get_vertex(&self, index: usize) -> Option<&Vertex<V>> {
        self.vertices.get(index)
    }

    pub fn get_vertex_mut(&mut self, index: usize) -> Option<&mut Vertex<V>> {
        self.vertices.get_mut(index)
    }

    pub fn get_vertices(&self) -> &Vec<Vertex<V>> {
        &self.vertices
    }

    pub fn get_edges(&self) -> &Vec<Edges<E>> {
        &self.edges
    }

    pub fn add_vertex(&mut self, data_vert: V, data_edge: E) {
        self.vertices.push(Vertex { data: data_vert });

        self.edges.push(Edges {
            inputs: Vec::new(),
            outputs: Vec::new(),
            data: data_edge,
        });
    }

    pub fn remove_vertex(&mut self, i: usize) {
        // The (original) index of the last vertex, which
        // will be swapped into the deleted vertex's place.
        let swapped_index = self.vertices.len() - 1;

        let removed_vertex = self.vertices.swap_remove(i);
        let removed_edges = self.edges.swap_remove(i);

        // Prune edges.
        for edges in self.edges.iter_mut() {
            // Delete edges that started at the removed vertex.
            edges.inputs.retain(|&index| index != i);

            // Delete edges that terminated at the removed vertex.
            edges.outputs.retain(|&index| index != i);

            // Update any edges that were pointing to or from the
            // swapped vertex.
            for index in edges.inputs.iter_mut() {
                if *index == swapped_index {
                    *index = i;
                }
            }
            for index in edges.outputs.iter_mut() {
                if *index == swapped_index {
                    *index = i;
                }
            }
        }
    }

    pub fn add_edge(&mut self, a: usize, b: usize) {
        self.edges[a].outputs.push(b);
        self.edges[b].inputs.push(a);
    }

    /// Performs a post-order traversal of the graph, returning
    /// the vertex indices in the proper order.
    pub fn traverse(&mut self, root: usize) -> Vec<usize> {
        let mut indices = Vec::new();
        let mut visited = Vec::new();

        // Traverse the graph, starting at the root.
        visited.push(root);
        self.recurse(root, &mut indices, &mut visited);

        indices
    }

    /// Examine a `root` op's inputs and recurse backwards until
    /// reaching a leaf node (i.e. an op with no other inputs).
    fn recurse(&self, root: usize, indices: &mut Vec<usize>, visited: &mut Vec<usize>) {
        for index in self.edges[root].inputs.iter() {
            self.recurse(*index, indices, visited);
        }

        // Finally, push back the root index.
        indices.push(root);
    }
}
