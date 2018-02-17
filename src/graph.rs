use std::cmp::max;

pub trait Connected {
    fn has_inputs(&self) -> bool;
    fn has_outputs(&self) -> bool;
    fn get_input_capacity(&self) -> usize;
    fn on_connect(&mut self);
    fn on_disconnect(&mut self);
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Node<T: Connected> {
    pub data: T,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Edges<T> {
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub data: T,
}

pub struct Graph<N: Connected, E> {
    /// The nodes (vertices) in the graph
    pub nodes: Vec<Node<N>>,

    /// A list of `Edges` structs, where each `Edges` corresponds
    /// to the node with the same index in `nodes`
    pub edges: Vec<Edges<E>>,
}

impl<N: Connected, E> Graph<N, E> {
    pub fn new() -> Graph<N, E> {
        Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn get_node(&self, index: usize) -> Option<&Node<N>> {
        self.nodes.get(index)
    }

    pub fn get_node_mut(&mut self, index: usize) -> Option<&mut Node<N>> {
        self.nodes.get_mut(index)
    }

    pub fn get_nodes(&self) -> &Vec<Node<N>> {
        &self.nodes
    }

    pub fn get_edges(&self) -> &Vec<Edges<E>> {
        &self.edges
    }

    pub fn add_node(&mut self, data_n: N, data_e: E) {
        self.nodes.push(Node { data: data_n });

        self.edges.push(Edges {
            inputs: Vec::new(),
            outputs: Vec::new(),
            data: data_e,
        });
    }

    pub fn remove_node(&mut self, i: usize) {
        // The (original) index of the last node, which
        // will be swapped into the deleted node's place.
        let swapped_index = self.nodes.len() - 1;

        let removed_vertex = self.nodes.swap_remove(i);
        let removed_edges = self.edges.swap_remove(i);

        // Prune edges.
        for edges in self.edges.iter_mut() {
            // Delete edges that started at the removed Node.
            edges.inputs.retain(|&index| index != i);

            // Delete edges that terminated at the removed Node.
            edges.outputs.retain(|&index| index != i);

            // Update any edges that were pointing to or from the
            // swapped Node.
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
        if a != b && self.nodes[a].data.has_outputs() && self.nodes[b].data.has_inputs() {
            self.nodes[a].data.on_connect();
            self.nodes[b].data.on_connect();

            self.edges[a].outputs.push(b);
            self.edges[b].inputs.push(a);
        } else {
            println!("Connection failed");
        }
    }

    /// Performs a post-order traversal of the graph, returning
    /// the node indices in the proper order.
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
