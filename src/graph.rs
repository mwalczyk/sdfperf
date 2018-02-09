use gl;
use gl::types::*;

use cgmath::{self, Matrix, Matrix4, One, PerspectiveFov, Point2, Vector2, Point3, Vector3, Vector4, Zero };
use uuid::Uuid;

use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;
use std::collections::HashSet;

use program::Program;
use operator::{Op, OpType, InteractionState};

type Color = Vector4<f32>;

pub struct Graph {
    pub ops: Vec<Op>,
    pub connections: HashSet<(Uuid, Uuid)>,
    render_program_operators: Program,
    render_program_connections: Program,
    render_projection_matrix: Matrix4<f32>,
    render_vao_operators: u32,
    render_vbo_operators: u32,
    render_vao_connections: u32,
    render_vbo_connections: u32,
    connections_point_cache: Vec<GLfloat>,
    connections_need_update: bool,
    network_zoom: f32,
    network_resolution: Vector2<f32>,

    pub root: Option<Uuid>,
    total_ops: usize
}

impl Graph {

    /// Constructs a new, empty graph.
    pub fn new() -> Graph {
        static VERTEX_DATA: [GLfloat; 12] = [
            // First triangle
            0.0, 0.0,   // UL
            1.0, 0.0,   // UR
            0.0, 1.0,   // LL

            // Second triangle
            1.0, 0.0,   // UR
            1.0, 1.0,   // LR
            0.0, 1.0    // LL
        ];

        static VS_SRC_OP: &'static str = "
        #version 430
        layout(location = 0) in vec2 position;
        uniform mat4 u_model_matrix;
        uniform mat4 u_projection_matrix;
        void main() {
            gl_Position = u_projection_matrix * u_model_matrix * vec4(position, 0.0, 1.0);
        }";

        static FS_SRC_OP: &'static str = "
        #version 430
        uniform vec4 u_draw_color;
        out vec4 o_color;
        void main() {
            o_color = u_draw_color;
        }";

        static VS_SRC_CN: &'static str = "
        #version 430
        layout(location = 0) in vec2 position;
        uniform mat4 u_projection_matrix;
        void main() {
            gl_Position = u_projection_matrix * vec4(position, 0.0, 1.0);
        }";

        static FS_SRC_CN: &'static str = "
        #version 430
        uniform vec4 u_draw_color;
        out vec4 o_color;
        void main() {
            o_color = vec4(1.0, 0.0, 0.2, 1.0);
        }";

        let render_program_operators = Program::new(VS_SRC_OP.to_string(), FS_SRC_OP.to_string());
        let render_program_connections = Program::new(VS_SRC_CN.to_string(), FS_SRC_CN.to_string());


        let mut render_vao_operators = 0;
        let mut render_vbo_operators = 0;
        let mut render_vao_connections = 0;
        let mut render_vbo_connections = 0;

        unsafe {
            // Set up OpenGL objects for rendering operators.
            gl::CreateBuffers(1, &mut render_vbo_operators);
            gl::NamedBufferData(
                render_vbo_operators,
                (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );

            let mut pos_attr = gl::GetAttribLocation(render_program_operators.program_id, CString::new("position").unwrap().as_ptr());

            gl::CreateVertexArrays(1, &mut render_vao_operators);
            gl::EnableVertexArrayAttrib(render_vao_operators, pos_attr as GLuint);
            gl::VertexArrayAttribFormat(render_vao_operators, pos_attr as GLuint, 2, gl::FLOAT, gl::FALSE as GLboolean, 0);
            gl::VertexArrayAttribBinding(render_vao_operators, pos_attr as GLuint, 0);

            gl::VertexArrayVertexBuffer(render_vao_operators, 0, render_vbo_operators, 0, (2 * mem::size_of::<GLfloat>()) as i32);

            // Set up OpenGL objects for rendering connections.
            pos_attr = gl::GetAttribLocation(render_program_connections.program_id, CString::new("position").unwrap().as_ptr());

            gl::CreateBuffers(1, &mut render_vbo_connections);
            gl::NamedBufferStorage(
                render_vbo_connections,
                (200 * mem::size_of::<GLfloat>()) as GLsizeiptr,
                ptr::null(),
                gl::DYNAMIC_STORAGE_BIT,
            );

            gl::CreateVertexArrays(1, &mut render_vao_connections);
            gl::EnableVertexArrayAttrib(render_vao_connections, pos_attr as GLuint);
            gl::VertexArrayAttribFormat(render_vao_connections, pos_attr as GLuint, 2, gl::FLOAT, gl::FALSE as GLboolean, 0);
            gl::VertexArrayAttribBinding(render_vao_connections, pos_attr as GLuint, 0);

            gl::VertexArrayVertexBuffer(render_vao_connections, 0, render_vbo_connections, 0, (2 * mem::size_of::<GLfloat>()) as i32);
        }

        let mut graph = Graph {
            ops: Vec::new(),
            connections: HashSet::new(),
            render_program_operators,
            render_program_connections,
            render_projection_matrix: Matrix4::zero(),
            render_vao_operators,
            render_vbo_operators,
            render_vao_connections,
            render_vbo_connections,
            connections_point_cache: Vec::new(),
            connections_need_update: false,
            network_zoom: 1.0,
            network_resolution: Vector2::new(800.0, 600.0),
            root: None,
            total_ops: 0
        };

        // Initialize the projection matrix.
        graph.set_network_zoom(1.0);

        graph
    }

    /// Zooms the network in or out by modifying the underlying
    /// projection matrix. If `network_zoom` is `1.0`, this is
    /// effectively the "home" position.
    pub fn set_network_zoom(&mut self, network_zoom: f32) {
        self.network_zoom = network_zoom;

        // Rebuild the projection matrix:
        // L, R, B, T, N, F
        self.render_projection_matrix = cgmath::ortho(-(self.network_resolution.x * 0.5) * self.network_zoom,
                                                      (self.network_resolution.x * 0.5) * self.network_zoom,
                                                      (self.network_resolution.y * 0.5) * self.network_zoom,
                                                      -(self.network_resolution.y * 0.5) * self.network_zoom,
                                                      -1.0,
                                                      1.0);
    }

    /// Adds a new op to the network at coordinates `screen_position`
    /// and dimensions `screen_size`.
    pub fn add_op(&mut self, screen_position: Vector2<f32>, screen_size: Vector2<f32>) {

        let op_type = match self.total_ops {
            0 => OpType::Sphere,
            1 => OpType::Render,
            _ => OpType::Render
        };

        self.total_ops += 1;
        println!("Total ops in network: {}", self.total_ops);
        println!("Adding op with type: {}", op_type.to_string());

        self.ops.push(Op::new(op_type, screen_position, screen_size));
    }

    /// Draws a single op in the network.
    fn draw_op(&self, op: &Op) {
        let mut model_matrix = op.region_operator.get_model_matrix();

        // Pick a draw color based on the current interaction state of this operator
        // and the op type.
        let mut draw_color = match op.state {
            InteractionState::Selected => Color::new(1.0, 1.0, 1.0, 1.0),
            InteractionState::Unselected => {
                match op.op_type {
                    OpType::Render => Color::new(1.0, 0.64, 0.44, 1.0),
                    _ => Color::new(0.5, 0.5, 0.5, 1.0)
                }
            },
            _ => Color::new(1.0, 1.0, 1.0, 1.0)
        };

        self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model_matrix);
        self.render_program_operators.uniform_4f("u_draw_color", &draw_color);

        unsafe {
            gl::BindVertexArray(self.render_vao_operators);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);

        }

        // Draw the connection slot(s), if necessary.
        match op.state {
            InteractionState::ConnectSource => {
                model_matrix = op.region_slot_output.get_model_matrix();
                draw_color = Color::new(0.0, 1.0, 0.1, 1.0);

                self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model_matrix);
                self.render_program_operators.uniform_4f("u_draw_color", &draw_color);

                unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
            },
            InteractionState::ConnectDestination => {
                model_matrix = op.region_slot_input.get_model_matrix();
                draw_color = Color::new(0.0, 1.0, 0.1, 1.0);

                self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model_matrix);
                self.render_program_operators.uniform_4f("u_draw_color", &draw_color);

                unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
            }
            _ => ()
        }
    }

    /// Draws all ops in the network.
    fn draw_all_ops(&mut self) {
        self.render_program_operators.bind();
        self.render_program_operators.uniform_matrix_4f("u_projection_matrix", &self.render_projection_matrix);
        for op in self.ops.iter() {
            self.draw_op(op);
        }
        self.render_program_operators.unbind();
    }

    /// Draws all connections between ops in the network.
    fn draw_all_connections(&mut self) {
        if self.connections_need_update {
            unsafe {
                println!("Current cache size: {}", self.connections_point_cache.len());

                // Update GPU data
                gl::NamedBufferSubData(
                    self.render_vbo_connections,
                    0,
                    (self.connections_point_cache.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                    self.connections_point_cache.as_ptr() as *const c_void
                );
            }
            self.connections_need_update = false;
        }

        self.render_program_connections.bind();
        self.render_program_connections.uniform_matrix_4f("u_projection_matrix", &self.render_projection_matrix);
        unsafe  {
            gl::BindVertexArray(self.render_vao_connections);
            gl::DrawArrays(gl::LINES, 0, self.connections_point_cache.len() as i32);
        }
        self.render_program_connections.unbind();
    }

    /// Draws all of the operators and connections that make
    /// up this graph.
    pub fn draw(&mut self) {
        self.draw_all_connections();
        self.draw_all_ops();
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
                    println!("Connected to render node: building graph");
                }

                // Deselect both ops.
                src.state = InteractionState::Unselected;
                dst.state = InteractionState::Unselected;

                // Push back the coordinates of the two connector slots.
                self.connections_point_cache.push(src_pt.x);
                self.connections_point_cache.push(src_pt.y);
                self.connections_point_cache.push(dst_pt.x);
                self.connections_point_cache.push(dst_pt.y);

                println!("Connections set: {:?}", self.connections);
                println!("-- Points: {:?} -> {:?}", src_pt, dst_pt);

                self.connections_need_update = true;
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

impl Drop for Graph {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.render_vbo_operators);
            gl::DeleteVertexArrays(1, &self.render_vao_operators);
        }
    }
}