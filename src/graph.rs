use cgmath::{self, Vector2, Vector4, Zero };
use uuid::Uuid;

use color::Color;
use operator::{Op, OpType, MouseInfo, InteractionState};
use renderer::Renderer;

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
struct Index(usize);

struct Node<T> {
    data: T,
    outputs: Vec<Index>,
    inputs: Vec<Index>
}

struct Arena<T> {
    nodes: Vec<Node<T>>
}

pub struct Graph {
    /// A memory arena that contains all of the ops
    pub ops: Vec<Op>,

    /// An adjacency list of connections between nodes
    pub connections: HashSet<(Uuid, Uuid)>,

    /// The UUID of the currently selected op (if there is one)
    pub selection: Option<Uuid>,

    /// The UUID of the root op (if there is one)
    pub root: Option<Uuid>,

    /// The total number of ops in the graph
    total_ops: usize,

    /// A flag that control whether or not the shader graph
    /// needs to be rebuilt
    dirty: bool
}

impl Graph {

    /// Constructs a new, empty graph.
    pub fn new() -> Graph {
        let graph = Graph {
            ops: Vec::new(),
            connections: HashSet::new(),
            selection: None,
            root: None,
            total_ops: 0,
            dirty: false
        };

        graph
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
        // Is there an op currently selected?
        if let Some(selected_uuid) = self.selection {

            let mut remove_index = None;
            for (index, op) in self.ops.iter().enumerate() {
                if op.id == selected_uuid {
                    remove_index = Some(index);
                    break;
                }
            }

            if let Some(index) = remove_index {
                self.ops.remove(index);

                for &(uuid_a, uuid_b) in &self.connections {

                }
            }

        }
    }

    /// Adds a new op of type `op_type` to the network at coordinates
    /// `screen_position` and dimensions `screen_size`.
    pub fn add_op(&mut self, op_type: OpType, position: Vector2<f32>, size: Vector2<f32>) {
        self.total_ops += 1;
        self.ops.push(Op::new(op_type, position, size));
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
        for &(uuid_a, uuid_b) in &self.connections {
            let op_a = self.get_op(uuid_a).unwrap();
            let op_b = self.get_op(uuid_b).unwrap();
            let centroid_a = op_a.aabb_slot_output.centroid();
            let centroid_b = op_b.aabb_slot_input.centroid();

            // Push back the first point.
            points.push(centroid_a.x);
            points.push(centroid_a.y);
            points.push(0.0);
            points.push(0.0);

            // Push back the second point.
            points.push(centroid_b.x);
            points.push(centroid_b.y);
            points.push(1.0);
            points.push(1.0);
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
    pub fn get_op(&self, uuid: Uuid) -> Option<&Op> {
        self.ops.iter().find(|op| op.id == uuid)
    }

    /// Returns an mutable reference to the op with the given
    /// UUID, if it exists in the graph.
    pub fn get_op_mut(&mut self, uuid: Uuid) -> Option<&mut Op> {
        self.ops.iter_mut().find(|op| op.id == uuid)
    }

    /// Adds a new connection between two ops with UUIDs
    /// `a` and `b`, respectively.
    pub fn add_connection(&mut self, uuid_a: Uuid, uuid_b: Uuid) {
        // First, find the two ops with matching UUIDs.
        let mut op_a: Option<&mut Op> = None;
        let mut op_b: Option<&mut Op> = None;
        for op in self.ops.iter_mut() {
            if op.id == uuid_a {
                op_a = Some(op);
            } else if op.id == uuid_b {
                op_b = Some(op);
            }
        }

        if let (Some(src), Some(dst)) = (op_a, op_b) {
            // Here, we only proceed if the connection was successful.
            if src.connect_to(dst) {

                // If we are connecting to a render op, then the shader
                // must be rebuilt.
                if dst.op_type == OpType::Render {
                    self.root = Some(dst.id);
                    self.dirty = true;
                    println!("Connected to render node: building graph");
                }

                // Deselect both ops.
                src.state = InteractionState::Deselected;
                dst.state = InteractionState::Deselected;
                self.connections.insert((uuid_a, uuid_b));

            } else {
                println!("Connection unsuccessful");
            }
        } else {
            println!("Attempting to connect two ops with non-existent UUIDs - something is wrong here")
        }
    }

    pub fn handle_interaction(&mut self, mouse_info: &MouseInfo) {
        let mut connecting = false;
        let mut src_id = Uuid::nil();
        let mut dst_id = Uuid::nil();

        for mut op in self.ops.iter_mut() {

            if let InteractionState::ConnectSource = op.state {
                if mouse_info.down {
                    // If this operator is currently being connected to another:
                    // 1) Set the `connecting` flag to `true`, as the user is
                    //    performing a potential op connection
                    // 2) Store its UUID as a potential connect source
                    // 3) Skip the rest of this loop iteration
                    connecting = true;
                    src_id = op.id;
                    continue;
                } else {
                    // Otherwise, deselect this op
                    op.state = InteractionState::Deselected;
                }
            }

            // Is the mouse inside of this op's bounding box?
            if op.aabb_op.inside(&mouse_info.curr) {

                // Is there an op currently selected?
                if let Some(uuid) = self.selection {

                    // Is this op the selected op?
                    if uuid == op.id  {

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
                        src_id = op.id;

                    } else {
                        // This op has been selected.
                        op.state = InteractionState::Selected;

                        // Store the selected UUID.
                        self.selection = Some(op.id);
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
                if let Some(uuid) = self.selection {

                    // Is this op the selected op?
                    if uuid == op.id {
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
            for op in self.ops.iter_mut() {

                // Make sure that the user is not trying to connect an operator to itself.
                if op.aabb_slot_input.inside_with_padding(&mouse_info.curr, 6.0) &&
                   src_id != op.id {

                    op.state = InteractionState::ConnectDestination;
                    dst_id = op.id;

                    // Only add the connection if:
                    // 1) It doesn't already exist in the hash set
                    // 2) The destination op actually accepts inputs
                    if !self.connections.contains(&(src_id, dst_id)) && op.op_type.has_inputs() {
                        found_new_connection = true;
                    }
                }

            }
        }

        if found_new_connection {
            self.add_connection(src_id, dst_id);
        }
    }
}
