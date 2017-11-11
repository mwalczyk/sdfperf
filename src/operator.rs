extern crate gl;

use cgmath::Vector2;
use cgmath::Matrix4;

use program::Program;
use gl::types::*;
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
        static VERTEX_DATA: [GLfloat; 6] = [ 0.0,  0.5,
            0.5, -0.5,
            -0.5, -0.5];

        static VS_SRC: &'static str = "
        #version 430
        layout(location = 0) in vec2 position;
        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }";

        static FS_SRC: &'static str = "
        #version 430
        uniform float u_time;
        out vec4 o_color;
        void main() {
            float pct = sin(u_time) * 0.5 + 0.5;

            o_color = vec4(pct, 1.0, 1.0, 1.0);
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

        for op in self.operators.iter() {
            unsafe {
                gl::BindVertexArray(self.render_vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 3);
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