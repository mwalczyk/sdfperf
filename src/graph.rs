use gl;
use gl::types::*;

use cgmath;
use cgmath::{ Matrix, Matrix4, One, PerspectiveFov, Point2, Vector2, Point3, Vector3, Vector4 };

use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;
use std::collections::HashSet;

use program::Program;
use operator::Operator;
use operator::InteractionState;

static NETWORK_WIDTH: f32 = 800.0;
static NETWORK_HEIGHT: f32 = 600.0;

pub struct Graph<'a> {
    operators: Vec<Operator>,
    connections: HashSet<(usize, usize)>,
    render_program_operators: Program<'a>,
    render_program_connections: Program<'a>,
    render_projection_matrix: Matrix4<f32>,
    render_vao: u32,
    render_vbo: u32,
    render_vao_connections: u32,
    render_vbo_connections: u32,
    connections_point_cache: Vec<GLfloat>,
    connections_need_update: bool,
    network_zoom: f32
}

impl<'a> Graph<'a> {

    pub fn new() -> Graph<'a> {
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

        let render_program_operators = Program::new(VS_SRC_OP, FS_SRC_OP);
        let render_program_connections = Program::new(VS_SRC_CN, FS_SRC_CN);

        // L, R, B, T, N, F
        let render_projection_matrix: Matrix4<f32> = cgmath::ortho(-(NETWORK_WIDTH * 0.5),
                                                                (NETWORK_WIDTH * 0.5),
                                                                (NETWORK_HEIGHT * 0.5),
                                                                -(NETWORK_HEIGHT * 0.5),
                                                                -1.0,
                                                                1.0);
        let mut render_vao = 0;
        let mut render_vbo = 0;
        let mut render_vao_connections = 0;
        let mut render_vbo_connections = 0;

        unsafe {
            // Set up GL objects for rendering operators
            gl::CreateBuffers(1, &mut render_vbo);
            gl::NamedBufferData(
                render_vbo,
                (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );

            let mut pos_attr = gl::GetAttribLocation(render_program_operators.program_id, CString::new("position").unwrap().as_ptr());

            gl::CreateVertexArrays(1, &mut render_vao);
            gl::EnableVertexArrayAttrib(render_vao, pos_attr as GLuint);
            gl::VertexArrayAttribFormat(render_vao, pos_attr as GLuint, 2, gl::FLOAT, gl::FALSE as GLboolean, 0);
            gl::VertexArrayAttribBinding(render_vao, pos_attr as GLuint, 0);

            gl::VertexArrayVertexBuffer(render_vao, 0, render_vbo, 0, (2 * mem::size_of::<GLfloat>()) as i32);

            // Set up GL objects for rendering connections
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

        Graph {
            operators: vec![],
            connections: HashSet::new(),
            render_program_operators,
            render_program_connections,
            render_projection_matrix,
            render_vao,
            render_vbo,
            render_vao_connections,
            render_vbo_connections,
            connections_point_cache: vec![],
            connections_need_update: false,
            network_zoom: 1.0
        }
    }

    pub fn set_network_zoom(&mut self, network_zoom: f32) {
        self.network_zoom = network_zoom;

        self.render_projection_matrix = cgmath::ortho(-(NETWORK_WIDTH * 0.5) * self.network_zoom,
                                                      (NETWORK_WIDTH * 0.5) * self.network_zoom,
                                                      (NETWORK_HEIGHT * 0.5) * self.network_zoom,
                                                      -(NETWORK_HEIGHT * 0.5) * self.network_zoom,
                                                      -1.0,
                                                      1.0);
    }

    pub fn add_operator(&mut self, screen_position: Vector2<f32>, screen_size: Vector2<f32>) {
        self.operators.push(Operator::new(screen_position, screen_size));
    }

    fn draw_operator(&self, op: &Operator) {
        let mut model_matrix = op.region_operator.get_model_matrix();

        // Pick a draw color based on the current interaction state of this operator
        let mut draw_color = match op.state {
            InteractionState::Selected => Vector4::new(1.0, 1.0, 1.0, 1.0),
            InteractionState::Unselected => Vector4::new(0.5, 0.5, 0.5, 1.0),
            _ => Vector4::new(1.0, 1.0, 1.0, 1.0)
        };

        self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model_matrix);
        self.render_program_operators.uniform_4f("u_draw_color", &draw_color);

        unsafe {
            gl::BindVertexArray(self.render_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        // Draw the connection slot, if necessary
        match op.state {
            InteractionState::ConnectSource => {
                model_matrix = op.region_slot_output.get_model_matrix();
                draw_color = Vector4::new(0.0, 1.0, 0.1, 1.0);

                self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model_matrix);
                self.render_program_operators.uniform_4f("u_draw_color", &draw_color);

                unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
            },
            InteractionState::ConnectDestination => {
                model_matrix = op.region_slot_input.get_model_matrix();
                draw_color = Vector4::new(0.0, 1.0, 0.1, 1.0);

                self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model_matrix);
                self.render_program_operators.uniform_4f("u_draw_color", &draw_color);

                unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
            }
            _ => ()
        }
    }

    fn draw_operators(&mut self) {
        self.render_program_operators.bind();
        self.render_program_operators.uniform_matrix_4f("u_projection_matrix", &self.render_projection_matrix);
        for op in self.operators.iter() {
            self.draw_operator(op);
        }
        self.render_program_operators.unbind();
    }

    fn draw_connections(&mut self) {
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
                gl::Flush();
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

    pub fn handle_interaction(&mut self, mouse_position: Vector2<f32>, mouse_down: bool) {
        // The user can only select a single operator at a time
        let mut found_selected = false;
        let mut connecting = false;
        let mut src_id: usize = 0;
        let mut dst_id: usize = 0;

        for (id, mut op) in self.operators.iter_mut().enumerate() {

            // If this operator is currently being connected to another,
            // skip the rest of this loop
            if let InteractionState::ConnectSource = op.state {
                if mouse_down {
                    found_selected = true;
                    connecting = true;
                    src_id = id;
                    continue;
                }
            }

            if op.region_operator.inside(&mouse_position) && !found_selected {

                // Otherwise, check to see if the user's mouse is within this
                // operator's output slot region
                if op.region_slot_output.inside_with_padding(&mouse_position, 6.0) && mouse_down  {
                    op.state = InteractionState::ConnectSource;
                    src_id = id;
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
        // is now over an input slot of a different operator)
        let mut found_new_connection = false;
        if connecting {
            for (id, op_destination) in self.operators.iter_mut().enumerate() {
                // Make sure that the user is not trying to connect an operator to itself
                if op_destination.region_slot_input.inside_with_padding(&mouse_position, 6.0) && src_id != id {
                    op_destination.state = InteractionState::ConnectDestination;
                    dst_id = id;

                    // Only add the connection if it doesn't already exist in the set
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
            let src_pt = self.operators[src_id].region_slot_output.centroid();
            let dst_pt = self.operators[dst_id].region_slot_input.centroid();
            self.operators[src_id].state = InteractionState::Unselected;
            self.operators[dst_id].state = InteractionState::Unselected;

            self.connections_point_cache.push(src_pt.x);
            self.connections_point_cache.push(src_pt.y);
            self.connections_point_cache.push(dst_pt.x);
            self.connections_point_cache.push(dst_pt.y);

            println!("Connections set: {:?}", self.connections);
            println!("-- Points: {:?} -> {:?}", src_pt, dst_pt);

            self.connections_need_update = true;
        }
    }

    pub fn draw(&mut self) {
        // Draw connections
        self.draw_connections();

        // Draw operators
        self.draw_operators();
    }
}

impl<'a> Drop for Graph<'a> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.render_vbo);
            gl::DeleteVertexArrays(1, &self.render_vao);
        }
    }
}