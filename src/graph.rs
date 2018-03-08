use std::cmp::max;

pub trait Connected {
    fn has_inputs(&self) -> bool;
    fn has_outputs(&self) -> bool;
    fn get_number_of_available_inputs(&self) -> usize;
    fn update_active_inputs_count(&mut self, count: usize);
    fn on_connect(&mut self);
    fn on_disconnect(&mut self);
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Node<T: Connected> {
    pub data: T,
}

impl<T: Connected> Node<T> {
    fn new(data: T) -> Node<T> {
        Node { data }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Edges<T> {
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub data: T,
}

impl<T> Edges<T> {
    fn new(data: T) -> Edges<T> {
        Edges {
            inputs: Vec::new(),
            outputs: Vec::new(),
            data,
        }
    }
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

    /// Returns an immutable reference to the node at `index`.
    pub fn get_node(&self, index: usize) -> Option<&Node<N>> {
        self.nodes.get(index)
    }

    /// Returns a mutable reference to the node at `index`.
    pub fn get_node_mut(&mut self, index: usize) -> Option<&mut Node<N>> {
        self.nodes.get_mut(index)
    }

    /// Returns an immutable reference to the graph's list of nodes.
    pub fn get_nodes(&self) -> &Vec<Node<N>> {
        &self.nodes
    }

    /// Returns a mutable reference to the graph's list of nodes.
    pub fn get_nodes_mut(&mut self) -> &mut Vec<Node<N>> {
        &mut self.nodes
    }

    pub fn get_edges(&self) -> &Vec<Edges<E>> {
        &self.edges
    }

    pub fn get_edges_mut(&mut self) -> &mut Vec<Edges<E>> {
        &mut self.edges
    }

    pub fn add_node(&mut self, data_n: N, data_e: E) {
        self.nodes.push(Node::new(data_n));
        self.edges.push(Edges::new(data_e));
    }

    pub fn remove_node(&mut self, i: usize) {
        // The (original) index of the last node, which
        // will be swapped into the deleted node's place.
        let swapped_index = self.nodes.len() - 1;

        let removed_vertex = self.nodes.swap_remove(i);
        let removed_edges = self.edges.swap_remove(i);

        // Prune edges.
        for (index, edges) in self.edges.iter_mut().enumerate() {
            // Delete edges that started at the removed node and
            // update the number of active inputs.
            edges.inputs.retain(|&input| input != i);
            let count = edges.inputs.len();
            self.nodes[index].data.update_active_inputs_count(count);

            // Delete edges that terminated at the removed node.
            edges.outputs.retain(|&output| output != i);

            // Update any edges that were pointing to or from the
            // swapped node.
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

    /// Removes the edge between nodes `a` and `b` (if it
    /// exists).
    pub fn remove_edge(&mut self, src: usize, dst: usize) {
        self.edges[src].outputs.retain(|&index| index != dst);
        self.edges[dst].inputs.retain(|&index| index != src);

        // Update the number of active inputs leading to
        // node `b`.
        let count = self.edges[dst].inputs.len();
        self.nodes[dst].data.update_active_inputs_count(count);
    }

    pub fn add_edge(&mut self, src: usize, dst: usize) {
        if src != dst && self.nodes[src].data.has_outputs() && self.nodes[dst].data.has_inputs() {
            // If node `b` has reached its input capacity, replace
            // the edge connecting its last input with `b` with
            // the new edge.
            if self.nodes[dst].data.get_number_of_available_inputs() == 0 {
                let old = self.edges[dst].inputs.pop().unwrap();
                self.remove_edge(old, dst);
            } else {

            }

            // Call the `on_connect` method for each node.
            self.nodes[dst].data.on_connect();

            // Update the edges.
            self.edges[src].outputs.push(dst);
            self.edges[dst].inputs.push(src);
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

        // Finally, push back the root index: note that
        // here we choose to ignore duplicate entries.
        // This occurs when a node is connected to multiple
        // nodes at varying depths in the graph.
        //
        // In other scenarios, we might want to allow this
        // insertion to happen, regardless if the index
        // exists in `indices` already.
        if !indices.contains(&root) {
            indices.push(root);
        }
    }
}
