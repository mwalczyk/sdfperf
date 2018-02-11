use cgmath::{self, Vector2, Vector4, Zero };
use uuid::Uuid;

use operator::{Op, OpType, MouseInfo, InteractionState};
use renderer::Renderer;

use std::collections::HashSet;

type Color = Vector4<f32>;

pub struct Graph {
    pub ops: Vec<Op>,

    pub connections: HashSet<(Uuid, Uuid)>,

    pub selection: Option<Uuid>,

    pub root: Option<Uuid>,

    total_ops: usize,

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

    /// Adds a new op of type `op_type` to the network at coordinates
    /// `screen_position` and dimensions `screen_size`.
    pub fn add_op(&mut self, op_type: OpType, position: Vector2<f32>, size: Vector2<f32>) {
        self.total_ops += 1;
        self.ops.push(Op::new(op_type, position, size));
        //println!("Op name: {}", self.ops.last().unwrap().name)
    }

    /// Pick a draw color based on the current interaction state of this
    /// operator and the op type.
    fn color_for_op(&self, op: &Op) -> Color {
        let mut color = match op.op_type {
            OpType::Sphere | OpType::Box | OpType::Plane => Color::new(0.77, 0.80, 1.0, 1.0),
            OpType::Union | OpType::Intersection | OpType::SmoothMinimum => Color::new(0.57, 0.60, 0.8, 1.0),
            OpType::Render => Color::new(0.99, 0.64, 0.45, 1.0)
        };

        // Add a contribution based on the op's current interaction state.
        color += match op.state {
            InteractionState::Hover => Color::new(0.05, 0.05, 0.05, 0.0),
            _ => Color::zero()
        };

        color
    }

    /// Draws a single op in the network.
    fn draw_op(&self, op: &Op, renderer: &Renderer) {
        // Draw the connection slot(s), if necessary.
        let slot_color = Color::new(0.31, 0.33, 0.48, 1.0);
        match op.state {
            InteractionState::Selected => {
                let aabb_select = op.aabb_op.expand_from_center(&Vector2::new(6.0, 6.0));
                renderer.draw_rect(&aabb_select, &Color::new(0.1, 1.0, 0.4, 1.0));
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

        let draw_color = Color::new(1.0, 1.0, 1.0, 1.0);

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
        for op in self.ops.iter() {
            if op.id == uuid {
                return Some(op);
            }
        }
        None
    }

    /// Returns an mutable reference to the op with the given
    /// UUID, if it exists in the graph.
    pub fn get_op_mut(&mut self, uuid: Uuid) -> Option<&mut Op> {
        for op in self.ops.iter_mut() {
            if op.id == uuid {
                return Some(op);
            }
        }
        None
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
                src.state = InteractionState::Unselected;
                dst.state = InteractionState::Unselected;

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
            // If this operator is currently being connected to another,
            // skip the rest of this loop.
            if let InteractionState::ConnectSource = op.state {
                if mouse_info.down {
                    connecting = true;
                    src_id = op.id;

                    continue;
                }
            }

            if op.aabb_op.inside(&mouse_info.curr) {
                // Is this op the last one that was selected?
                if let Some(uuid) = self.selection {
                    if uuid == op.id  {
                        if mouse_info.down {
                            let velocity = mouse_info.curr - mouse_info.last;
                            op.aabb_op.translate(&velocity);
                            op.aabb_slot_input.translate(&velocity);
                            op.aabb_slot_output.translate(&velocity);
                        }
                        continue;
                    }
                }

                // Is the mouse down?
                if mouse_info.down {
                    // Are we inside the bounds of this op's output slot?
                    if op.aabb_slot_output.inside_with_padding(&mouse_info.curr, 12.0) {
                        op.state = InteractionState::ConnectSource;

                        // Store the connection source UUID.
                        src_id = op.id;
                    } else {
                        op.state = InteractionState::Selected;

                        // Store the UUID of the op that was selected.
                        self.selection = Some(op.id);
                    }
                }
                else {
                    // Otherwise, the mouse is still inside the bounds of this op,
                    // so we must be hovering over it.
                    op.state = InteractionState::Hover;
                }
            }
            else {
                if let Some(uuid) = self.selection {
                    if uuid != op.id {
                        op.state = InteractionState::Unselected;
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

                    // Only add the connection if it doesn't already exist in the hash set
                    // and the destination op actually accepts inputs.
                    if !self.connections.contains(&(src_id, dst_id)) &&
                        op.op_type.has_inputs() {
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
