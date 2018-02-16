use gl;
use gl::types::*;
use cgmath::{self, Matrix, Matrix4, One, PerspectiveFov, SquareMatrix, Vector2, Vector4, Zero};

use bounding_rect::BoundingRect;
use color::Color;
use program::Program;

use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;
use std::time::{Duration, SystemTime};

pub struct Renderer {
    /// The shader program that will be used to draw sprites
    draw_program: Program,

    /// The projection matrix used to render the network orthographically
    projection: Matrix4<f32>,

    /// The VAO that contains vertex attribute descriptions for sprite
    /// rendering
    vao: u32,

    /// The VBO that contains the vertex data necessary for rendering
    /// rectangular sprites
    vbo_rect: u32,

    /// The VBO that will be dynamically updated with vertex data
    /// for rendering lines
    vbo_line: u32,

    /// The zoom of the network editor
    network_zoom: f32,

    /// The resolution (in pixels) of the network editor
    network_resolution: Vector2<f32>,

    /// The user-generated shader program that will be built dynamically
    preview_program: Option<Program>,

    /// The program used if the `preview_program` is unassigned or invalid
    fallback_program: Program,

    /// THe AABB of the SDF render view
    aabb_preview: BoundingRect,

    /// An application timer
    time: SystemTime,
}

impl Renderer {
    /// Constructs and returns a new renderer instance.
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

        static COMMON_VS_SRC: &'static str = "
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

        static DRAW_FS_SRC: &'static str = "
        #version 430

        uniform float u_time;
        uniform vec4 u_draw_color = vec4(1.0);
        uniform uint u_draw_mode = 0;

        layout (location = 0) in vec2 vs_texcoord;
        layout (location = 0) out vec4 o_color;

        void main() {
            vec2 uv = vs_texcoord;

            float alpha = 1.0;
            switch(u_draw_mode)
            {
            case 0:
                alpha = 1.0;
                break;
            case 1:
                alpha = step(0.5, fract(uv.s * 20.0 - u_time)) + 0.4;
                break;
            }
            o_color = vec4(u_draw_color.rgb, alpha);
        }";

        static FALLBACK_FS_SRC: &'static str = "
        #version 430

        layout (location = 0) in vec2 vs_texcoord;
        layout (location = 0) out vec4 o_color;

        void main() {
            const float tile = 10.0;
            vec2 uv = vs_texcoord * tile;
            vec2 ipos = floor(uv);

            float total = dot(ipos, vec2(1.0));
            float checkerboard = mod(total, 2.0);

            o_color = vec4(vec3(checkerboard), 1.0);;
        }";

        // Compile the shader program.
        let draw_program =
            Program::new(COMMON_VS_SRC.to_string(), DRAW_FS_SRC.to_string()).unwrap();
        let fallback_program =
            Program::new(COMMON_VS_SRC.to_string(), FALLBACK_FS_SRC.to_string()).unwrap();

        // Setup buffers.
        let mut vao = 0;
        let mut vbo_rect = 0;
        let mut vbo_line = 0;
        unsafe {
            // Enable alpha blending.
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // Create the VBO for rendering rectangles.
            let vbo_rect_size = (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr;
            gl::CreateBuffers(1, &mut vbo_rect);
            gl::NamedBufferData(
                vbo_rect,
                vbo_rect_size,
                mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );

            // Create the VBO for rendering lines.
            let vbo_line_size = (200 * mem::size_of::<GLfloat>()) as GLsizeiptr;
            gl::CreateBuffers(1, &mut vbo_line);
            gl::NamedBufferStorage(
                vbo_line,
                vbo_line_size,
                ptr::null(),
                gl::DYNAMIC_STORAGE_BIT,
            );

            // This is not strictly necessary, but we do it for completeness sake.
            let pos_attr = gl::GetAttribLocation(
                draw_program.program_id,
                CString::new("position").unwrap().as_ptr(),
            );
            let tex_attr = gl::GetAttribLocation(
                draw_program.program_id,
                CString::new("texcoord").unwrap().as_ptr(),
            );
            let tex_offset = (2 * mem::size_of::<GLfloat>()) as GLuint;

            // Create the VAO and setup vertex attributes.
            gl::CreateVertexArrays(1, &mut vao);

            // Position attribute.
            gl::EnableVertexArrayAttrib(vao, pos_attr as GLuint);
            gl::VertexArrayAttribFormat(
                vao,
                pos_attr as GLuint,
                2,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                0,
            );
            gl::VertexArrayAttribBinding(vao, pos_attr as GLuint, 0);

            // Texture coordinates attribute.
            gl::EnableVertexArrayAttrib(vao, tex_attr as GLuint);
            gl::VertexArrayAttribFormat(
                vao,
                tex_attr as GLuint,
                2,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                tex_offset,
            );
            gl::VertexArrayAttribBinding(vao, tex_attr as GLuint, 0);

            // Associate the VBO with bind point 0.
            gl::VertexArrayVertexBuffer(
                vao,
                0,
                vbo_rect,
                0,
                (4 * mem::size_of::<GLfloat>()) as i32,
            );
        }

        let mut renderer = Renderer {
            draw_program,
            projection: Matrix4::zero(),
            vao,
            vbo_rect,
            vbo_line,
            network_zoom: 1.0,
            network_resolution: Vector2::new(800.0, 600.0),
            preview_program: None,
            fallback_program,
            aabb_preview: BoundingRect::new(Vector2::new(100.0, 000.0), Vector2::new(300.0, 300.0)),
            time: SystemTime::now(),
        };

        renderer.zoom(1.0);

        renderer
    }

    /// Zooms the network in or out by modifying the underlying
    /// projection matrix. If `zoom` is `1.0`, this is
    /// effectively the "home" position.
    pub fn zoom(&mut self, zoom: f32) {
        self.network_zoom = zoom;
        self.rebuild_projection_matrix();
    }

    /// Resizes the network.
    pub fn resize(&mut self, resolution: &Vector2<f32>) {
        self.network_resolution = *resolution;
        self.rebuild_projection_matrix();
    }

    /// Rebuild the projection matrix:
    /// L, R, B, T, N, F
    fn rebuild_projection_matrix(&mut self) {
        self.projection = cgmath::ortho(
            -(self.network_resolution.x * 0.5) * self.network_zoom,
            (self.network_resolution.x * 0.5) * self.network_zoom,
            (self.network_resolution.y * 0.5) * self.network_zoom,
            -(self.network_resolution.y * 0.5) * self.network_zoom,
            -1.0,
            1.0,
        );
    }

    /// Sets the shader program that will be used to render a
    /// miniature preview window in the lower right-hand corner
    /// of the network.
    ///
    /// If `program` is `None`, then the renderer will use a
    /// fall-back shader to indicate the error state of the
    /// current graph.
    pub fn set_preview_program(&mut self, program: Option<Program>) {
        self.preview_program = program;
    }

    /// If a preview program has be assigned, render a miniature
    /// preview window in the lower right-hand corner of the
    /// network.
    pub fn draw_preview(&self) {
        // First, set all relevant uniforms.
        let model = self.aabb_preview.get_model_matrix();

        if let Some(ref program) = self.preview_program {
            program.bind();
            program.uniform_matrix_4f("u_model_matrix", &model);
            program.uniform_matrix_4f("u_projection_matrix", &self.projection);
        } else {
            self.fallback_program.bind();
            self.fallback_program
                .uniform_matrix_4f("u_model_matrix", &model);
            self.fallback_program
                .uniform_matrix_4f("u_projection_matrix", &self.projection);
        }

        // Next, issue a draw call.
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        if let Some(ref program) = self.preview_program {
            program.unbind();
        } else {
            self.fallback_program.unbind();
        }
    }

    /// Draws the rectangle described by `rect`, with solid `color`.
    pub fn draw_rect(&self, rect: &BoundingRect, color: &Color) {
        self.draw_program.bind();

        // First, set all relevant uniforms.
        let model = rect.get_model_matrix();
        self.draw_program
            .uniform_matrix_4f("u_model_matrix", &model);
        self.draw_program
            .uniform_matrix_4f("u_projection_matrix", &self.projection);
        self.draw_program
            .uniform_4f("u_draw_color", &(*color).into());
        self.draw_program.uniform_1ui("u_draw_mode", 0);
        self.draw_program
            .uniform_1f("u_time", self.get_elapsed_seconds());

        // Next, issue a draw call.
        unsafe {
            gl::VertexArrayVertexBuffer(
                self.vao,
                0,
                self.vbo_rect,
                0,
                (4 * mem::size_of::<GLfloat>()) as i32,
            );

            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        self.draw_program.unbind();
    }

    /// Draws a series of line segments.
    pub fn draw_line(&self, data: &Vec<f32>, color: &Color) {
        self.draw_program.bind();

        // First, set all relevant uniforms.
        let model = Matrix4::identity();
        self.draw_program
            .uniform_matrix_4f("u_model_matrix", &model);
        self.draw_program
            .uniform_matrix_4f("u_projection_matrix", &self.projection);
        self.draw_program
            .uniform_4f("u_draw_color", &(*color).into());
        self.draw_program.uniform_1ui("u_draw_mode", 1);
        self.draw_program
            .uniform_1f("u_time", self.get_elapsed_seconds());

        // Next, update buffer storage and issue a draw call.
        unsafe {
            let data_size = (data.len() * mem::size_of::<GLfloat>()) as GLsizeiptr;
            gl::NamedBufferSubData(self.vbo_line, 0, data_size, data.as_ptr() as *const c_void);

            gl::VertexArrayVertexBuffer(
                self.vao,
                0,
                self.vbo_line,
                0,
                (4 * mem::size_of::<GLfloat>()) as i32,
            );

            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::LINES, 0, (data.len() / 4) as i32);
        }

        self.draw_program.unbind();
    }

    fn get_elapsed_seconds(&self) -> f32 {
        let elapsed = self.time.elapsed().unwrap();
        let milliseconds = elapsed.as_secs() * 1000 + elapsed.subsec_nanos() as u64 / 1_000_000;

        (milliseconds as f32) / 1000.0
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo_rect);
            gl::DeleteBuffers(1, &self.vbo_line);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
