use cgmath::{self, Vector2, Vector4, Zero };
use uuid::Uuid;

use color::Color;
use operator::{Op, OpType, OpIndex, MouseInfo, InteractionState};
use renderer::Renderer;

use std::cmp::max;
use std::collections::HashSet;

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
pub enum OpPair<'a> {
    Both(&'a mut Op, &'a mut Op),
    One(&'a mut Op),
    None,
}

pub struct Graph {
    /// A memory arena that contains all of the ops
    pub ops: Vec<Op>,

    /// An adjacency list of connections between nodes
    pub connections: HashSet<(OpIndex, OpIndex)>,

    /// The index of the currently selected op (if there is one)
    pub selection: Option<OpIndex>,

    /// The index of the root op (if there is one)
    pub root: Option<OpIndex>,

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

impl Graph {

    /// Constructs a new, empty graph.
    pub fn new() -> Graph {
        Graph {
            ops: Vec::new(),
            connections: HashSet::new(),
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

            println!("Connections before: {:?}", self.connections);

            // Only retain connections that did not lead to/from the
            // removed op.
            self.connections.retain(|&(src, dst)| {
                src != selected && dst != selected
            });

            // Remove the op.
            let selected_op = self.ops.swap_remove(selected.0);

            println!("Connections after: {:?}", self.connections);

            // Reset the selection.
            self.selection = None;

            // Update the index of the op that was swapped into the old
            // op's location in the memory arena.
            self.ops[selected.0].index = selected;

            // Rebuild the connections involving the last_op

            // Remove all connections that led to the removed op.
         }
    }

    /// Adds a new op of type `op_type` to the network at coordinates
    /// `screen_position` and dimensions `screen_size`.
    pub fn add_op(&mut self, op_type: OpType, position: Vector2<f32>, size: Vector2<f32>) {
        let index = OpIndex(self.ops.len());
        self.ops.push(Op::new(index, op_type,position, size));
    }

    /// Pick a draw color based on the current interaction state of this
    /// operator and the op type.
    fn color_for_op(&self, op: &Op) -> Color {
        let mut color = match op.op_type {
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
        for op in self.ops.iter() {
            self.draw_op(op, renderer);
        }
    }

    /// Draws all connections between ops in the network.
    fn draw_all_connections(&self, renderer: &Renderer) {
        let mut points = Vec::new();

        for &(src, dst) in &self.connections {
            if let (Some(src_op), Some(dst_op)) = (self.get_op(src), self.get_op(dst)) {
                let src_centroid = src_op.aabb_slot_output.centroid();
                let dst_centroid = dst_op.aabb_slot_input.centroid();

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

    /// Draws all of the operators and connections that make
    /// up this graph.
    pub fn draw(&mut self, renderer: &Renderer) {
        self.draw_all_connections(renderer);
        self.draw_all_ops(renderer);
    }

    /// Returns an immutable reference to the op with the given
    /// UUID, if it exists in the graph.
    pub fn get_op(&self, index: OpIndex) -> Option<&Op> {
        self.ops.get(index.0)
    }

    /// Returns an mutable reference to the op with the given
    /// UUID, if it exists in the graph.
    pub fn get_op_mut(&mut self, index: OpIndex) -> Option<&mut Op> {
        self.ops.get_mut(index.0)
    }

    /// Adds a new connection between two ops with UUIDs
    /// `a` and `b`, respectively.
    pub fn add_connection(&mut self, src: OpIndex, dst: OpIndex) {
        if let Pair::Both(src_op, dst_op) = index_twice(&mut self.ops, src.0, dst.0) {

            // Here, we only proceed if the connection was successful.
            if src_op.connect_to(dst_op) {

                // If we are connecting to a render op, then the shader
                // must be rebuilt.
                if dst_op.op_type == OpType::Render {
                    self.root = Some(dst_op.index);
                    self.dirty = true;
                    println!("Connected to render node: building graph");
                }

                // Deselect both ops.
                src_op.state = InteractionState::Deselected;
                dst_op.state = InteractionState::Deselected;

                // Add the new connection.
                self.connections.insert((src, dst));
            }
        } else {
            println!("Attempting to connect two ops with the same index - something is wrong here")
        }

        println!("Connections after: {:?}", self.connections);
    }

    pub fn handle_interaction(&mut self, mouse_info: &MouseInfo) {
        let mut connecting = false;
        let mut src = OpIndex(0);
        let mut dst = OpIndex(0);

        for (index, op) in self.ops.iter_mut().enumerate() {

            if let InteractionState::ConnectSource = op.state {
                if mouse_info.down {
                    // If this operator is currently being connected to another:
                    // 1) Set the `connecting` flag to `true`, as the user is
                    //    performing a potential op connection
                    // 2) Store its UUID as a potential connect source
                    // 3) Skip the rest of this loop iteration
                    connecting = true;
                    src = OpIndex::from(index);
                    continue;
                } else {
                    // Otherwise, deselect this op
                    op.state = InteractionState::Deselected;
                }
            }

            // Is the mouse inside of this op's bounding box?
            if op.aabb_op.inside(&mouse_info.curr) {

                // Is there an op currently selected?
                if let Some(selected) = self.selection {

                    // Is this op the selected op?
                    if selected == OpIndex::from(index)  {

                        // Is the mouse down?
                        if mouse_info.down {
                            op.translate(&(mouse_info.curr - mouse_info.last));
                        }
                        continue;
                    }
                }

                // This op is not the selected op, but we are inside of it's
                // bounding box. Is the mouse down?
                if mouse_info.down {

                    // Are we inside the bounds of this op's output slot?
                    if op.aabb_slot_output.inside_with_padding(&mouse_info.curr, 12.0) {
                        // This op is now a potential connection source.
                        op.state = InteractionState::ConnectSource;

                        // Store the connection source UUID.
                        src = OpIndex::from(index);

                    } else {
                        // This op has been selected.
                        op.state = InteractionState::Selected;

                        // Store the selected UUID.
                        self.selection = Some(OpIndex::from(index));
                    }
                }
                else {
                    // Otherwise, the mouse is still inside the bounds of this op,
                    // so we must be hovering over it.
                    op.state = InteractionState::Hover;
                }

            // The mouse is not inside of this op's bounding box.
            } else {

                // Is there an op currently selected?
                if let Some(selected) = self.selection {

                    // Is this op the selected op?
                    if selected == OpIndex::from(index) {
                        // Keep this op selected.
                        op.state = InteractionState::Selected;
                    } else {
                        // Deselect the op.
                        op.state = InteractionState::Deselected;
                    }
                }
            }
        }

        // If the mouse is dragging from the output slot of one operator,
        // check if a potential connection has happened (i.e. the mouse
        // is now over an input slot of a different operator).
        let mut found_new_connection = false;

        if connecting {
            for (index, op) in self.ops.iter_mut().enumerate() {

                // Make sure that the user is not trying to connect an operator to itself.
                if op.aabb_slot_input.inside_with_padding(&mouse_info.curr, 6.0) && src != OpIndex::from(index) {
                    op.state = InteractionState::ConnectDestination;
                    dst = OpIndex::from(index);

                    // Only add the connection if the destination op actually accepts inputs
                    if op.op_type.has_inputs() {
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
