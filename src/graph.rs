use cgmath::{self, Vector2, Vector4, Zero };
use uuid::Uuid;

use std::collections::HashSet;

use operator::{Op, OpType, InteractionState};
use renderer::Renderer;

type Color = Vector4<f32>;

pub struct Graph {
    pub ops: Vec<Op>,

    pub connections: HashSet<(Uuid, Uuid)>,

    pub connections_point_cache: Vec<f32>,

    pub root: Option<Uuid>,

    pub total_ops: usize,

    dirty: bool
}

impl Graph {

    /// Constructs a new, empty graph.
    pub fn new() -> Graph {
        let mut graph = Graph {
            ops: Vec::new(),
            connections: HashSet::new(),
            connections_point_cache: Vec::new(),
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

    /// Adds a new op to the network at coordinates `screen_position`
    /// and dimensions `screen_size`.
    pub fn add_op(&mut self, position: Vector2<f32>, size: Vector2<f32>) {
        let op_type = match self.total_ops {
            0 => OpType::Sphere,
            1 => OpType::Box,
            2 => OpType::SmoothMinimum,
            _ => OpType::Render
        };

        self.total_ops += 1;
        println!("Total ops in network: {}", self.total_ops);
        println!("Adding op with type: {}", op_type.to_string());

        self.ops.push(Op::new(op_type, position, size));
        println!("Op name: {}", self.ops.last().unwrap().name)
    }

    /// Draws a single op in the network.
    fn draw_op(&self, op: &Op, renderer: &Renderer) {
        // Pick a draw color based on the current interaction state of this operator
        // and the op type.
        let mut draw_color = match op.op_type {
            OpType::Sphere | OpType::Box | OpType::Plane => Color::new(0.77, 0.80, 1.0, 1.0),
            OpType::Union | OpType::Intersection | OpType::SmoothMinimum => Color::new(0.57, 0.60, 0.8, 1.0),
            OpType::Render => Color::new(0.99, 0.64, 0.45, 1.0)
        };

        draw_color += match op.state {
            InteractionState::Selected => Color::new(0.05, 0.05, 0.05, 0.0),
            _ => Color::zero()
        };

       renderer.draw_rect(&op.region_operator, &draw_color);

        // Draw the connection slot(s), if necessary.
        draw_color = Color::new(0.31, 0.33, 0.48, 1.0);
        match op.state {
            InteractionState::ConnectSource => renderer.draw_rect(&op.region_slot_output, &draw_color),
            InteractionState::ConnectDestination => renderer.draw_rect(&op.region_slot_input, &draw_color),
            _ => ()
        }
    }

    /// Draws all ops in the network.
    fn draw_all_ops(&mut self, renderer: &Renderer) {
        for op in self.ops.iter() {
            self.draw_op(op, renderer);
        }
    }

    /// Draws all connections between ops in the network.
    fn draw_all_connections(&mut self, renderer: &Renderer) {
        let draw_color = Color::new(1.0, 1.0, 1.0, 1.0);
        renderer.draw_line(&self.connections_point_cache, &draw_color);
    }

    /// Draws all of the operators and connections that make
    /// up this graph.
    pub fn draw(&mut self, renderer: &Renderer) {
        self.draw_all_connections(renderer);
        self.draw_all_ops(renderer);
    }

    /// Returns an immutable reference to the op with the given
    /// UUID, if it exists in the graph.
    pub fn get_op(&self, id: Uuid) -> Option<&Op> {
        for op in self.ops.iter() {
            if op.id == id {
                return Some(op);
            }
        }
        None
    }

    /// Returns an mutable reference to the op with the given
    /// UUID, if it exists in the graph.
    pub fn get_op_mut(&mut self, id: Uuid) -> Option<&mut Op> {
        for op in self.ops.iter_mut() {
            if op.id == id {
                return Some(op);
            }
        }
        None
    }

    /// Adds a new connection between two ops with UUIDs
    /// `a` and `b`, respectively.
    pub fn add_connection(&mut self, a: Uuid, b: Uuid) {
        // First, find the two ops with matching UUIDs.
        let mut op_a: Option<&mut Op> = None;
        let mut op_b: Option<&mut Op> = None;
        for op in self.ops.iter_mut() {
            if op.id == a {
                op_a = Some(op);
            }
            else if op.id == b {
                op_b = Some(op);
            }
        }

        if let (Some(src), Some(dst)) = (op_a, op_b) {
            let src_pt = src.region_slot_output.centroid();
            let dst_pt = dst.region_slot_input.centroid();

            if src.connect_to(dst) {
                // Here, we only proceed if the connection was successful.
                if dst.op_type == OpType::Render {
                    self.root = Some(dst.id);
                    self.dirty = true;
                    println!("Connected to render node: building graph");
                }

                // Deselect both ops.
                src.state = InteractionState::Unselected;
                dst.state = InteractionState::Unselected;

                // Push back the coordinates of the two connector slots: note
                // that we also add texture coordinates here so that the layouts
                // of the two buffers match.
                self.connections_point_cache.push(src_pt.x);
                self.connections_point_cache.push(src_pt.y);
                self.connections_point_cache.push(0.0);
                self.connections_point_cache.push(0.0);

                self.connections_point_cache.push(dst_pt.x);
                self.connections_point_cache.push(dst_pt.y);
                self.connections_point_cache.push(1.0);
                self.connections_point_cache.push(1.0);
            } else {
                println!("Connection unsuccessful");
            }
        } else {
            println!("Attempting to connect two ops with non-existent UUIDs - something is wrong here")
        }
    }

    pub fn handle_interaction(&mut self, mouse_position: Vector2<f32>, mouse_down: bool) {
        // The user can only select a single operator at a time.
        let mut found_selected = false;
        let mut connecting = false;
        let mut src_id = Uuid::nil();
        let mut dst_id = Uuid::nil();

        for mut op in self.ops.iter_mut() {

            // If this operator is currently being connected to another,
            // skip the rest of this loop.
            if let InteractionState::ConnectSource = op.state {
                if mouse_down {
                    found_selected = true;
                    connecting = true;
                    src_id = op.id;

                    continue;
                }
            }

            if op.region_operator.inside(&mouse_position) && !found_selected {
                // Otherwise, check to see if the user's mouse is within this
                // operator's output slot region.
                if op.region_slot_output.inside_with_padding(&mouse_position, 6.0) && mouse_down  {
                    op.state = InteractionState::ConnectSource;
                    src_id = op.id;
                }
                else {
                    op.state = InteractionState::Selected;
                }
                found_selected = true;
            }
            else {
                op.state = InteractionState::Unselected;
            }
        }

        // If the mouse is dragging from the output slot of one operator,
        // check if a potential connection has happened (i.e. the mouse
        // is now over an input slot of a different operator).
        let mut found_new_connection = false;
        if connecting {
            for op_destination in self.ops.iter_mut() {
                // Make sure that the user is not trying to connect an operator to itself.
                if op_destination.region_slot_input.inside_with_padding(&mouse_position, 6.0) &&
                   src_id != op_destination.id {

                    op_destination.state = InteractionState::ConnectDestination;

                    dst_id = op_destination.id;

                    // Only add the connection if it doesn't already exist in the hash set.
                    if !self.connections.contains(&(src_id, dst_id)) {
                        self.connections.insert((src_id, dst_id));
                        found_new_connection = true;
                    }
                    else {
                        println!("Hash set already contains the ID pair: {:?}", (src_id, dst_id));
                    }
                }

            }
        }

        if found_new_connection {
            self.add_connection(src_id, dst_id);
        }
    }
}
