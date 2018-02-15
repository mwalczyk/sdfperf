use cgmath::{self, Vector2, Vector4, Zero };
use uuid::Uuid;

use color::Color;
use graph::Graph;
use operator::{Op, OpType, MouseInfo, InteractionState};
use renderer::Renderer;

use std::cmp::max;

/// Palette:
///
/// Background:  0x2B2B2B (dark gray)
/// Accent:      0x373737 (light gray)
/// Generator:   0x8F719D (purple)
/// Combiner:    0xA8B6C5 (blue)
/// Render:      0xC77832 (orange)
/// Selection:   0x76B264 (green)
/// Other:       0xFEC56D (yellow)
///

type Connection = usize;

pub struct Network {
    /// An adjacency list representation of ops
    pub graph: Graph<Op, Connection>,

    /// The index of the currently selected op (if there is one)
    pub selection: Option<usize>,

    /// The index of the root op (if there is one)
    pub root: Option<usize>,

    /// A flag that control whether or not the shader graph
    /// needs to be rebuilt
    dirty: bool
}

enum Pair<T> {
    Both(T, T),
    One(T),
    None,
}

/// Get mutable references at index `a` and `b`.
/// See: https://stackoverflow.com/questions/30073684/how-to-get-mutable-references-to-two-array-elements-at-the-same-time
fn index_twice<T>(slc: &mut [T], a: usize, b: usize) -> Pair<&mut T> {
    if max(a, b) >= slc.len() {
        Pair::None
    } else if a == b {
        Pair::One(&mut slc[max(a, b)])
    } else {
        unsafe {
            let ar = &mut *(slc.get_unchecked_mut(a) as *mut _);
            let br = &mut *(slc.get_unchecked_mut(b) as *mut _);
            Pair::Both(ar, br)
        }
    }
}

impl Network {

    /// Constructs a new, empty network.
    pub fn new() -> Network {
        Network {
            graph: Graph::new(),
            selection: None,
            root: None,
            dirty: false
        }
    }

    /// Returns `true` if the shader graph needs to be rebuilt and
    /// `false` otherwise.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Sets the `dirty` flag to `false`.
    pub fn clean(&mut self) {
        self.dirty = false;
    }

    pub fn delete_selected(&mut self) {
        if let Some(selected) = self.selection {
            self.graph.remove_vertex(selected);
            self.selection = None;
            println!("Number of vertices: {}", self.graph.vertices.len());
            println!("Number of edge lists: {}", self.graph.edges.len());
        }
    }

    /// Adds a new op of type `op_type` to the network at coordinates
    /// `screen_position` and dimensions `screen_size`.
    pub fn add_op(&mut self, family: OpType, position: Vector2<f32>, size: Vector2<f32>) {
        let op = Op::new(family, position, size);
        self.graph.add_vertex(op, 0);
    }

    /// Pick a draw color based on the current interaction state of this
    /// operator and the op type.
    fn color_for_op(&self, op: &Op) -> Color {
        let mut color = match op.family {
            OpType::Sphere | OpType::Box | OpType::Plane => Color::from_hex(0x8F719D),
            OpType::Union | OpType::Intersection | OpType::SmoothMinimum => Color::from_hex(0xA8B6C5),
            OpType::Render => Color::from_hex(0xC77832)
        };

        // Add a contribution based on the op's current interaction state.
        color += match op.state {
            InteractionState::Hover => Color::mono(0.05,0.0),
            _ => Color::black()
        };

        color
    }

    /// Draws a single op in the network.
    fn draw_op(&self, op: &Op, renderer: &Renderer) {
        // Draw the op and other components:
        // - If the op is selected, draw a selection box behind it
        // - If the op is being used as a connection source or
        //   destination, draw the appropriate connection slot
        let slot_color = Color::from_hex(0x373737);

        match op.state {
            InteractionState::Selected => {
                let aabb_select = op.aabb_op.expand_from_center(&Vector2::new(6.0, 6.0));
                renderer.draw_rect(&aabb_select, &Color::from_hex(0x76B264));
            },
            InteractionState::ConnectSource => renderer.draw_rect(&op.aabb_slot_output, &slot_color),
            InteractionState::ConnectDestination => renderer.draw_rect(&op.aabb_slot_input, &slot_color),
            _ => ()
        }

        renderer.draw_rect(&op.aabb_op, &self.color_for_op(op));
    }

    /// Draws all ops in the network.
    fn draw_all_ops(&mut self, renderer: &Renderer) {
        for vertex in self.graph.get_vertices().iter() {
            self.draw_op(&vertex.data, renderer);
        }
    }

    /// Draws all edges between ops in the network.
    fn draw_all_edges(&self, renderer: &Renderer) {
        let mut points = Vec::new();

        for (src, edges) in self.graph.edges.iter().enumerate() {

            for dst in edges.outputs.iter() {

                let src_vert = self.graph.get_vertex(src).unwrap();
                let dst_vert = self.graph.get_vertex(*dst).unwrap();

                let src_centroid = src_vert.data.aabb_slot_output.centroid();
                let dst_centroid = dst_vert.data.aabb_slot_input.centroid();

                // Push back the first point.
                points.push(src_centroid.x);
                points.push(src_centroid.y);
                points.push(0.0);
                points.push(0.0);

                // Push back the second point.
                points.push(dst_centroid.x);
                points.push(dst_centroid.y);
                points.push(1.0);
                points.push(1.0);
            }
        }

        let draw_color = Color::white();

        renderer.draw_line(&points, &draw_color);
    }

    /// Draws all of the operators and edges that make
    /// up this graph.
    pub fn draw(&mut self, renderer: &Renderer) {
        self.draw_all_edges(renderer);
        self.draw_all_ops(renderer);
    }

    /// Adds a new connection between two ops.
    pub fn add_connection(&mut self, a: usize, b: usize) {

        self.graph.add_edge(a, b);

        if let Pair::Both(vert_a, vert_b) = index_twice(&mut self.graph.vertices, a, b) {
            // If we are connecting to a render op, then the shader
            // must be rebuilt.
            if  vert_b.data.family == OpType::Render {
                self.root = Some(b);
                self.dirty = true;
                println!("Connected to render node: building graph");
            }

            // Deselect both ops.
            vert_a.data.state = InteractionState::Deselected;
            vert_b.data.state = InteractionState::Deselected;

        } else {
            println!("Attempting to connect two ops with the same index - something is wrong here")
        }
    }

    pub fn handle_interaction(&mut self, mouse_info: &MouseInfo) {
        let mut connecting = false;
        let mut src: usize = 0;
        let mut dst: usize = 0;

        for (index, vertex) in self.graph.vertices.iter_mut().enumerate() {

            if let InteractionState::ConnectSource = vertex.data.state {
                if mouse_info.down {
                    // If this operator is currently being connected to another:
                    // 1) Set the `connecting` flag to `true`, as the user is
                    //    performing a potential op connection
                    // 2) Store its UUID as a potential connect source
                    // 3) Skip the rest of this loop iteration
                    connecting = true;
                    src = index;
                    continue;
                } else {
                    // Otherwise, deselect this op
                    vertex.data.state = InteractionState::Deselected;
                }
            }

            // Is the mouse inside of this op's bounding box?
            if vertex.data.aabb_op.inside(&mouse_info.curr) {

                // Is there an op currently selected?
                if let Some(selected) = self.selection {

                    // Is this op the selected op?
                    if selected == index  {

                        // Is the mouse down?
                        if mouse_info.down {
                            vertex.data.translate(&(mouse_info.curr - mouse_info.last));
                        }
                        continue;
                    }
                }

                // This op is not the selected op, but we are inside of it's
                // bounding box. Is the mouse down?
                if mouse_info.down {

                    // Are we inside the bounds of this op's output slot?
                    if vertex.data.aabb_slot_output.inside_with_padding(&mouse_info.curr, 12.0) {
                        // This op is now a potential connection source.
                        vertex.data.state = InteractionState::ConnectSource;

                        // Store the connection source index.
                        src = index;

                    } else {
                        // This op has been selected.
                        vertex.data.state = InteractionState::Selected;

                        // Store the selected UUID.
                        self.selection = Some(index);
                    }
                }
                else {
                    // Otherwise, the mouse is still inside the bounds of this op,
                    // so we must be hovering over it.
                    vertex.data.state = InteractionState::Hover;
                }

            // The mouse is not inside of this op's bounding box.
            } else {

                // Is there an op currently selected?
                if let Some(selected) = self.selection {

                    // Is this op the selected op?
                    if selected == index {
                        // Keep this op selected.
                        vertex.data.state = InteractionState::Selected;
                    } else {
                        // Deselect the op.
                        vertex.data.state = InteractionState::Deselected;
                    }
                } else {
                    // Deselect the op.
                    vertex.data.state = InteractionState::Deselected;
                }
            }
        }

        // If the mouse is dragging from the output slot of one operator,
        // check if a potential connection has happened (i.e. the mouse
        // is now over an input slot of a different operator).
        let mut found_new_connection = false;

        if connecting {
            for (index, vertex) in self.graph.vertices.iter_mut().enumerate() {

                // Make sure that the user is not trying to connect an operator to itself.
                if vertex.data.aabb_slot_input.inside_with_padding(&mouse_info.curr, 6.0) && src != index {
                    vertex.data.state = InteractionState::ConnectDestination;
                    dst = index;

                    // Only add the connection if:
                    // 1) The destination op actually accepts inputs
                    // 2) The connection doesn't already exist
                    if vertex.data.family.has_inputs() && !self.graph.edges[src].outputs.contains(&dst) {
                        found_new_connection = true;
                    }
                }

            }
        }

        if found_new_connection {
            self.add_connection(src, dst);
        }
    }
}
