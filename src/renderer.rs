use gl;
use gl::types::*;
use cgmath::{self, Matrix, Matrix4, One, PerspectiveFov, SquareMatrix, Vector2, Vector4, Zero};

use bounding_rect::BoundingRect;
use color::Color;
use program::Program;
use texture::Texture;

use std::mem;
use std::ptr;
use std::os::raw::c_void;
use std::ffi::CString;
use std::time::{Duration, SystemTime};

#[derive(Copy, Clone)]
pub enum LineMode {
    Solid,
    Dashed,
}

#[derive(Clone)]
pub enum DrawParams<'a> {
    Rectangle(&'a BoundingRect),
    Line(&'a Vec<f32>, LineMode),
}

pub struct Renderer {
    /// The OpenGL handle of the currently bound program (if there is one)
    bound_programs: Vec<GLuint>,

    /// The OpenGL handle of the currently bound VAO (if there is one)
    bound_vao: Option<GLuint>,

    /// The shader program that will be used to draw sprites
    program_draw: Program,

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

        static DRAW_VS_SRC: &'static str = "
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

        layout(binding = 0) uniform sampler2D u_color_map;
        layout(binding = 1) uniform sampler2D u_alpha_map;
        uniform bool u_use_color_map;
        uniform bool u_use_alpha_map;

        layout (location = 0) in vec2 vs_texcoord;
        layout (location = 0) out vec4 o_color;

        const uint DRAW_MODE_RECTANGLES = 0;
        const uint DRAW_MODE_LINES_SOLID = 1;
        const uint DRAW_MODE_LINES_DASHED = 2;
        void main() {
            vec2 uv = vs_texcoord;

            float alpha = u_draw_color.a;;
            if (u_draw_mode == DRAW_MODE_LINES_DASHED)
            {
                const float stripes = 20.0;
                alpha = max(step(0.5, fract(uv.s * stripes - u_time)), 0.5);
            }

            // the alpha map overrides the default alpha
            if (u_use_alpha_map)
            {
                alpha = texture(u_alpha_map, uv).r;
            }

            if (u_use_color_map)
            {
                uv.t = 1.0 - uv.t;
                vec4 color = texture(u_color_map, uv);
                color.a *= alpha;
                o_color = color;
            }
            else
            {
                o_color = vec4(u_draw_color.rgb, alpha);
            }
        }";

        // Compile the shader program.
        let program_draw = Program::new(DRAW_VS_SRC.to_string(), DRAW_FS_SRC.to_string()).unwrap();

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
            let vbo_line_size = (1000 * mem::size_of::<GLfloat>()) as GLsizeiptr;
            gl::CreateBuffers(1, &mut vbo_line);
            gl::NamedBufferStorage(
                vbo_line,
                vbo_line_size,
                ptr::null(),
                gl::DYNAMIC_STORAGE_BIT,
            );

            // This is not strictly necessary, but we do it for completeness sake.
            let pos_attr =
                gl::GetAttribLocation(program_draw.id, CString::new("position").unwrap().as_ptr());
            let tex_attr =
                gl::GetAttribLocation(program_draw.id, CString::new("texcoord").unwrap().as_ptr());
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
            bound_programs: Vec::new(),
            bound_vao: None,
            program_draw,
            projection: Matrix4::zero(),
            vao,
            vbo_rect,
            vbo_line,
            network_zoom: 1.0,
            network_resolution: Vector2::new(800.0, 600.0),
            time: SystemTime::now(),
        };

        renderer.zoom(1.0);

        renderer
    }

    pub fn get_resolution(&self) -> &Vector2<f32> {
        &self.network_resolution
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

        // Set the uniform.
        self.program_draw
            .uniform_matrix_4f("u_projection_matrix", &self.projection);
    }

    pub fn draw(
        &mut self,
        params: DrawParams,
        color: &Color,
        color_map: Option<&Texture>,
        alpha_map: Option<&Texture>,
    ) {
        self.program_draw.bind();

        let mut model = Matrix4::identity();
        if let DrawParams::Rectangle(bounds) = params {
            model = *bounds.get_model_matrix();
        }

        // Set shared uniforms.
        self.program_draw
            .uniform_matrix_4f("u_model_matrix", &model);
        self.program_draw
            .uniform_4f("u_draw_color", &(*color).into());
        self.program_draw
            .uniform_1f("u_time", self.get_elapsed_seconds());

        // Bind the color map, if available.
        if let Some(color_map) = color_map {
            self.program_draw.uniform_1i("u_use_color_map", true as i32);
            color_map.bind(0);
        } else {
            self.program_draw
                .uniform_1i("u_use_color_map", false as i32);
        }

        // Bind the alpha map, if available.
        if let Some(alpha_map) = alpha_map {
            self.program_draw.uniform_1i("u_use_alpha_map", true as i32);
            alpha_map.bind(1);
        } else {
            self.program_draw
                .uniform_1i("u_use_alpha_map", false as i32);
        }

        match params {
            DrawParams::Rectangle(_) => {
                self.program_draw.uniform_1ui("u_draw_mode", 0);
                self.draw_rect_inner();
            }
            DrawParams::Line(data, mode) => {
                self.program_draw
                    .uniform_1ui("u_draw_mode", mode as u32 + 1);
                self.draw_line_inner(&data);
            }
        }

        self.program_draw.unbind();
    }

    fn draw_rect_inner(&self) {
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
    }

    fn draw_line_inner(&self, data: &Vec<f32>) {
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
    }

    /// Draws the rectangle described by `rect`, with solid `color`.
    pub fn draw_rect_with_program(&self, rect: &BoundingRect, program: &Program) {
        program.bind();

        // First, set all relevant uniforms.
        program.uniform_matrix_4f("u_model_matrix", &rect.get_model_matrix());
        program.uniform_matrix_4f("u_projection_matrix", &self.projection);
        program.uniform_1f("u_time", self.get_elapsed_seconds());

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

        program.unbind();
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
