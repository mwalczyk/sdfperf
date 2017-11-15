use gl;
use gl::types::*;

use cgmath;
use cgmath::{ Matrix, Matrix4, One, PerspectiveFov, Point2, Vector2, Point3, Vector3, Vector4 };

use std::mem;
use std::ffi::CString;

use program::Program;
use operator::Operator;
use operator::InteractionState;

pub struct Graph<'a> {
    operators: Vec<Operator>,
    render_program: Program<'a>,
    render_vao: u32,
    render_vbo: u32,
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

        static VS_SRC: &'static str = "
        #version 430
        layout(location = 0) in vec2 position;
        uniform mat4 u_model_matrix;
        uniform mat4 u_projection_matrix;
        void main() {
            gl_Position = u_projection_matrix * u_model_matrix * vec4(position, 0.0, 1.0);
        }";

        static FS_SRC: &'static str = "
        #version 430
        uniform vec4 u_draw_color;
        uniform float u_time;
        out vec4 o_color;
        void main() {
            o_color = u_draw_color;
        }";

        let render_program = Program::new(VS_SRC, FS_SRC);
        let mut render_vao = 0;
        let mut render_vbo = 0;

        unsafe {
            gl::CreateBuffers(1, &mut render_vbo);
            gl::NamedBufferData(
                render_vbo,
                (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );

            let pos_attr = gl::GetAttribLocation(render_program.program_id, CString::new("position").unwrap().as_ptr());

            gl::CreateVertexArrays(1, &mut render_vao);
            gl::EnableVertexArrayAttrib(render_vao, pos_attr as GLuint);
            gl::VertexArrayAttribFormat(
                render_vao,
                pos_attr as GLuint,
                2,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                0
            );
            gl::VertexArrayAttribBinding(render_vao, pos_attr as GLuint, 0);

            gl::VertexArrayVertexBuffer(render_vao, 0, render_vbo, 0, (2 * mem::size_of::<GLfloat>()) as i32);
        }

        Graph {
            operators: vec![],
            render_program,
            render_vao,
            render_vbo,
            network_zoom: 1.0
        }
    }

    pub fn set_network_zoom(&mut self, network_zoom: f32) {
        self.network_zoom = network_zoom;
    }

    pub fn add_operator(&mut self, screen_position: Vector2<f32>, screen_size: Vector2<f32>) {
        self.operators.push(Operator::new(screen_position, screen_size));
    }

    fn build_model_matrix(&self, op: &Operator) -> Matrix4<f32> {
        let translation = Matrix4::from_translation(
            Vector3::new(
                op.region_operator.upper_left.x,
                op.region_operator.upper_left.y,
                0.0)
        );

        let scale = Matrix4::from_nonuniform_scale(
            op.region_operator.size.x,
            op.region_operator.size.y,
            0.0
        );

        translation * scale
    }

    fn build_slot_model_matrix(&self, op: &Operator) -> Matrix4<f32> {
        static SLOT_DRAW_SIZE: (f32, f32) = (10.0, 10.0);

        let translation = Matrix4::from_translation(
            Vector3::new(
                op.region_slot.upper_left.x,
                op.region_slot.upper_left.y,
                0.0)
        );

        let scale = Matrix4::from_nonuniform_scale(
            op.region_slot.size.x,
            op.region_slot.size.x,
            0.0
        );

        translation * scale
    }

    fn draw_operator(&self, op: &Operator) {
        let mut model_matrix = self.build_model_matrix(op);

        // Pick a draw color based on the current interaction state of this operator
        let mut draw_color = match op.state {
            InteractionState::Selected => Vector4::new(1.0, 1.0, 1.0, 1.0),
            InteractionState::Unselected => Vector4::new(0.5, 0.5, 0.5, 1.0),
            _ => Vector4::new(1.0, 1.0, 1.0, 1.0)
        };

        self.render_program.uniform_matrix_4f("u_model_matrix", &model_matrix);
        self.render_program.uniform_4f("u_draw_color", &draw_color);

        unsafe {
            gl::BindVertexArray(self.render_vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        // Draw the connection slot, if necessary
        if let InteractionState::ConnectSource = op.state {
            model_matrix = self.build_slot_model_matrix(op);
            draw_color = Vector4::new(0.0, 1.0, 0.1, 1.0);

            self.render_program.uniform_matrix_4f("u_model_matrix", &model_matrix);
            self.render_program.uniform_4f("u_draw_color", &draw_color);

            unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
        }
    }

    pub fn check_selected(&mut self, position: Vector2<f32>, mouse_down: bool) {
        // The user can only select a single operator at a time
        let mut found_selected = false;

        for op in self.operators.iter_mut() {
            // If this operator is currently being connected to another,
            // skip the rest of this loop
            if let InteractionState::ConnectSource = op.state {
                if mouse_down {
                    found_selected = true;
                    continue;
                }
            }

            if op.region_operator.inside(&position) && !found_selected {

                // Otherwise, check to see if the user's mouse is within this
                // operator's output slot region
                if op.region_slot.inside(&position) && mouse_down  {
                    op.state = InteractionState::ConnectSource;
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
    }

    pub fn draw(&self) {
        self.render_program.bind();

        // L, R, B, T, N, F
        const NETWORK_WIDTH: f32 = 800.0;
        const NETWORK_HEIGHT: f32 = 600.0;
        let projection_matrix: Matrix4<f32> = cgmath::ortho(-(NETWORK_WIDTH * 0.5) * self.network_zoom,
                                                            (NETWORK_WIDTH * 0.5) * self.network_zoom,
                                                            (NETWORK_HEIGHT * 0.5) * self.network_zoom,
                                                            -(NETWORK_HEIGHT * 0.5) * self.network_zoom,
                                                            -1.0,
                                                            1.0);

        self.render_program.uniform_matrix_4f("u_projection_matrix", &projection_matrix);

        for op in self.operators.iter() {
            self.draw_operator(op);
        }

        self.render_program.unbind();
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