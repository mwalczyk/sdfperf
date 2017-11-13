extern crate gl;
extern crate cgmath;

use program::Program;

use gl::types::*;
use cgmath::{ Matrix, Matrix4, One, PerspectiveFov, Point2, Vector2, Point3, Vector3 };

use std::mem;
use std::ffi::CString;

struct Operator {
    screen_position: Vector2<f32>,
    screen_size: Vector2<f32>
}

impl Operator {

    fn new(screen_position: Vector2<f32>, screen_size: Vector2<f32>) -> Operator {
        Operator {
            screen_position,
            screen_size
        }
    }

   fn draw(&self) {


    }
}

pub struct Graph<'a> {
    operators: Vec<Operator>,
    render_program: Program<'a>,
    render_vao: u32,
    render_vbo: u32
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
        out vec4 o_color;
        void main() {
            o_color = vec4(1.0, 1.0, 1.0, 1.0);
        }";

        let operators: Vec<Operator> = vec![];
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
            operators,
            render_program,
            render_vao,
            render_vbo
        }
    }

    pub fn add_operator(&mut self, screen_position: Vector2<f32>, screen_size: Vector2<f32>) {
        self.operators.push(Operator::new(screen_position, screen_size));
    }

    pub fn draw(&self) {
        self.render_program.bind();



        // L, R, B, T, N, F
        let projection: Matrix4<f32> = cgmath::ortho(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);

        self.render_program.unifrom_matrix_4fv("u_projection_matrix", &projection);

        for op in self.operators.iter() {
            let translation = Matrix4::from_translation(
                Vector3::new(
                    op.screen_position.x,
                    op.screen_position.y,
                    0.0)
            );

            let scale = Matrix4::from_nonuniform_scale(
                    op.screen_size.x,
                    op.screen_size.y,
                    0.0
            );
            let model = translation * scale;

            self.render_program.unifrom_matrix_4fv("u_model_matrix", &model);

            unsafe {
                gl::BindVertexArray(self.render_vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }
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