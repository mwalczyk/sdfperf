use gl;
use gl::types::*;
use cgmath::{self, Matrix, Matrix4, One, PerspectiveFov, Point2, Vector2, Point3, Vector3, Vector4, Zero };

use bounding_rect::BoundingRect;
use program::Program;

use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;

type Color = Vector4<f32>;

pub struct Renderer {
    program: Program,
    projection: Matrix4<f32>,
    vao: u32,
    vbo: u32,
    network_zoom: f32,
    network_resolution: Vector2<f32>
}

impl Renderer {

    pub fn new() -> Renderer {
        static VERTEX_DATA: [GLfloat; 24] = [
            // Positions followed by texture coordinates.
            // First triangle
            0.0, 0.0,   0.0, 1.0, // UL
            1.0, 0.0,   1.0, 1.0, // UR
            0.0, 1.0,   0.0, 0.0, // LL

            // Second triangle
            1.0, 0.0,   1.0, 1.0, // UR
            1.0, 1.0,   1.0, 0.0, // LR
            0.0, 1.0,   0.0, 0.0  // LL
        ];

        static VS_SRC: &'static str = "
        #version 430

        layout(location = 0) in vec2 position;
        layout(location = 1) in vec2 texcoord;

        layout (location = 0) out vec2 vs_texcoord;

        uniform mat4 u_model_matrix;
        uniform mat4 u_projection_matrix;

        void main() {
            vs_texcoord = texcoord;

            gl_Position = u_projection_matrix * u_model_matrix * vec4(position, 0.0, 1.0);
        }";

        static FS_SRC: &'static str = "
        #version 430

        uniform vec4 u_draw_color = vec4(1.0);

        layout (location = 0) in vec2 vs_texcoord;

        layout (location = 0) out vec4 o_color;

        void main() {
            o_color = u_draw_color;
        }";

        // Compile the shader program.
        let program = Program::new(VS_SRC.to_string(), FS_SRC.to_string());

        // Setup buffers.
        let mut vao = 0;
        let mut vbo = 0;
        unsafe {
            let vbo_size = (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr;

            gl::CreateBuffers(1, &mut vbo);
            gl::NamedBufferData(vbo, vbo_size, mem::transmute(&VERTEX_DATA[0]), gl::STATIC_DRAW);

            // This is not strictly necessary, but we do it for completeness sake.
            let pos_attr = gl::GetAttribLocation(program.program_id, CString::new("position").unwrap().as_ptr());
            let tex_attr = gl::GetAttribLocation(program.program_id, CString::new("texcoord").unwrap().as_ptr());
            let tex_offset = (2 * mem::size_of::<GLfloat>()) as GLuint;

            // Create the VAO and setup vertex attributes.
            gl::CreateVertexArrays(1, &mut vao);

            // Position attribute.
            gl::EnableVertexArrayAttrib(vao, pos_attr as GLuint);
            gl::VertexArrayAttribFormat(vao, pos_attr as GLuint, 2, gl::FLOAT, gl::FALSE as GLboolean, 0);
            gl::VertexArrayAttribBinding(vao, pos_attr as GLuint, 0);

            // Texture coordinates attribute.
            gl::EnableVertexArrayAttrib(vao, tex_attr as GLuint);
            gl::VertexArrayAttribFormat(vao, tex_attr as GLuint, 2, gl::FLOAT, gl::FALSE as GLboolean, tex_offset);
            gl::VertexArrayAttribBinding(vao, tex_attr as GLuint, 0);

            // Associate the VBO with bind point 0.
            gl::VertexArrayVertexBuffer(vao, 0, vbo, 0, (4 * mem::size_of::<GLfloat>()) as i32);
        }

        let mut renderer = Renderer {
            program,
            projection: Matrix4::zero(),
            vao,
            vbo,
            network_zoom: 1.0,
            network_resolution: Vector2::new(800.0, 600.0)
        };

        renderer.zoom(1.0);

        renderer
    }

    /// Zooms the network in or out by modifying the underlying
    /// projection matrix. If `zoom` is `1.0`, this is
    /// effectively the "home" position.
    pub fn zoom(&mut self, zoom: f32) {
        self.network_zoom = zoom;

        // Rebuild the projection matrix:
        // L, R, B, T, N, F
        self.projection = cgmath::ortho(-(self.network_resolution.x * 0.5) * self.network_zoom,
                                        (self.network_resolution.x * 0.5) * self.network_zoom,
                                        (self.network_resolution.y * 0.5) * self.network_zoom,
                                        -(self.network_resolution.y * 0.5) * self.network_zoom,
                                        -1.0,
                                        1.0);
    }

    pub fn resize(&mut self, resolution: &Vector2<f32>) {
        // TODO
    }

    pub fn draw_rect(&self, rect: &BoundingRect, color: &Color) {
        self.program.bind();

        // First, set all relevant uniforms.
        let model = rect.get_model_matrix();
        self.render_program_operators.uniform_matrix_4f("u_model_matrix", &model);
        self.render_program_operators.uniform_matrix_4f("u_projection_matrix", &self.projection);
        self.render_program_operators.uniform_4f("u_draw_color", &color);

        // Next, issue a draw call.
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        self.program.unbind();
    }

    pub fn draw_line(&self) {

    }
}