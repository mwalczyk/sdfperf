use cgmath::{self, Vector2, Vector4, Zero};
use uuid::Uuid;

use color::Color;
use graph::Graph;
use interaction::{InteractionState, MouseInfo};
use operator::{Op, OpType};
use preview::Preview;
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

    /// The preview of the shader that is represented by the
    /// current network
    pub preview: Preview,

    /// A flag that controls whether or not the shader graph
    /// needs to be rebuilt
    dirty: bool,

    /// A flag that controls whether or not the preview will
    /// be drawn
    show_preview: bool
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
            preview: Preview::new(),
            dirty: false,
            show_preview: true
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

    /// Toggles drawing of the preview window.
    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
    }

    /// Deletes the currently selected op (if the selection is not empty).
    pub fn delete_selected(&mut self) {
        if let Some(selected) = self.selection {
            // Before removing this vertex from the graph,
            // check to see if it is connected to the root
            // (if one exists). If so, then the shader
            // graph needs to be rebuilt.
            if let Some(root) = self.root {
                for edge in self.graph.edges[selected].outputs.iter() {
                    if *edge == root {
                        self.dirty = true;
                        self.root = None;
                        break;
                    }
                }
            }

            self.graph.remove_node(selected);
            self.selection = None;
            println!("Number of vertices: {}", self.graph.nodes.len());
            println!("Number of edge lists: {}", self.graph.edges.len());
        }
    }

    /// Adds a new op of type `op_type` to the network at coordinates
    /// `screen_position` and dimensions `screen_size`.
    pub fn add_op(&mut self, family: OpType, position: Vector2<f32>, size: Vector2<f32>) {
        let op = Op::new(family, position, size);
        self.graph.add_node(op, 0);
    }

    /// Pick a draw color based on the current interaction state of this
    /// operator and the op type.
    fn color_for_op(&self, op: &Op) -> Color {
        let mut color = match op.family {
            OpType::Sphere | OpType::Box | OpType::Plane => Color::from_hex(0x8F719D),
            OpType::Union | OpType::Intersection | OpType::SmoothMinimum => {
                Color::from_hex(0xA8B6C5)
            }
            OpType::Render => Color::from_hex(0xC77832),
        };

        // Add a contribution based on the op's current interaction state.
        color += match op.state {
            InteractionState::Hover => Color::mono(0.05, 0.0),
            _ => Color::black(),
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
            }
            InteractionState::ConnectSource => {
                renderer.draw_rect(&op.aabb_slot_output, &slot_color)
            }
            InteractionState::ConnectDestination => {
                renderer.draw_rect(&op.aabb_slot_input, &slot_color)
            }
            _ => (),
        }

        renderer.draw_rect(&op.aabb_op, &self.color_for_op(op));
    }

    /// Draws all ops in the network.
    fn draw_all_ops(&mut self, renderer: &Renderer) {
        for node in self.graph.get_nodes().iter() {
            self.draw_op(&node.data, renderer);
        }
    }

    /// Draws all edges between ops in the network.
    fn draw_all_edges(&self, renderer: &Renderer) {
        let mut points = Vec::new();

        for (src, edges) in self.graph.edges.iter().enumerate() {
            for dst in edges.outputs.iter() {
                let src_node = self.graph.get_node(src).unwrap();
                let dst_node = self.graph.get_node(*dst).unwrap();

                let src_centroid = src_node.data.aabb_slot_output.centroid();
                let dst_centroid = dst_node.data.aabb_slot_input.centroid();

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

        if self.show_preview {
            self.preview.draw(renderer);
        }
    }

    /// Adds a new connection between two ops.
    pub fn add_connection(&mut self, a: usize, b: usize) {
        self.graph.add_edge(a, b);

        if let Pair::Both(node_a, node_b) = index_twice(&mut self.graph.nodes, a, b) {
            // If we are connecting to a render op, then the shader
            // must be rebuilt.
            if node_b.data.family == OpType::Render {
                self.root = Some(b);
                self.dirty = true;
                println!("Connected to render node: building graph");
            }

            // Deselect both ops.
            node_a.data.state = InteractionState::Deselected;
            node_b.data.state = InteractionState::Deselected;
        } else {
            println!("Attempting to connect two ops with the same index - something is wrong here")
        }
    }

    pub fn handle_interaction(&mut self, mouse: &MouseInfo) {
        let mut connecting = false;
        let mut src: Option<usize> = None;
        let mut dst: Option<usize> = None;

        for (index, node) in self.graph.nodes.iter_mut().enumerate() {
            if let InteractionState::ConnectSource = node.data.state {
                if mouse.down {
                    // If this operator is currently being connected to another:
                    // 1) Set the `connecting` flag to `true`, as the user is
                    //    performing a potential op connection
                    // 2) Store its graph index as a potential connect source
                    // 3) Skip the rest of this loop iteration
                    connecting = true;
                    src = Some(index);
                    continue;
                } else {
                    // Otherwise, deselect this op
                    node.data.state = InteractionState::Deselected;
                }
            }

            // Is the mouse inside of this op's bounding box?
            if node.data.aabb_op.inside(&mouse.curr) {
                // Is there an op currently selected?
                if let Some(selected) = self.selection {
                    // Is this op the selected op?
                    if selected == index {
                        // Is the mouse down?
                        if mouse.down {
                            node.data.translate(&(mouse.curr - mouse.last));
                        }
                        continue;
                    }
                }

                // This op is not the selected op, but we are inside of it's
                // bounding box. Is the mouse down?
                if mouse.down {
                    // Are we inside the bounds of this op's output slot?
                    if node.data
                        .aabb_slot_output
                        .inside_with_padding(&mouse.curr, 12.0)
                    {
                        // This op is now a potential connection source.
                        node.data.state = InteractionState::ConnectSource;

                        // Store the connection source index.
                        src = Some(index);
                    } else {
                        // This op has been selected.
                        node.data.state = InteractionState::Selected;

                        // Store the selected UUID.
                        self.selection = Some(index);
                    }
                } else {
                    // Otherwise, the mouse is still inside the bounds of this op,
                    // so we must be hovering over it.
                    node.data.state = InteractionState::Hover;
                }

            // The mouse is not inside of this op's bounding box.
            } else {
                // Is there an op currently selected?
                if let Some(selected) = self.selection {
                    // Is this op the selected op?
                    if selected == index {
                        // Keep this op selected.
                        node.data.state = InteractionState::Selected;
                    } else {
                        // Deselect the op.
                        node.data.state = InteractionState::Deselected;
                    }
                } else {
                    // Deselect the op.
                    node.data.state = InteractionState::Deselected;
                }
            }
        }

        // If the mouse is dragging from the output slot of one operator,
        // check if a potential connection has happened (i.e. the mouse
        // is now over an input slot of a different operator).
        if connecting {
            for (index, node) in self.graph.nodes.iter_mut().enumerate() {
                // Is the mouse now inside of a different op's input slot region?
                if node.data
                    .aabb_slot_input
                    .inside_with_padding(&mouse.curr, 12.0)
                {
                    node.data.state = InteractionState::ConnectDestination;
                    if let Some(src) = src {
                        dst = Some(index);
                    }
                }
            }
        }

        if let (Some(src), Some(dst)) = (src, dst) {
            self.add_connection(src, dst);
        }

        self.preview.handle_interaction(&mouse);
    }
}
