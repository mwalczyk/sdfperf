use cgmath::{Matrix4, SquareMatrix, Vector2};

use bounding_rect::BoundingRect;
use interaction::MouseInfo;
use program::Program;
use renderer::Renderer;

pub struct Preview {
    program_valid: Option<Program>,

    program_error: Program,

    aabb: BoundingRect,

    lookat: Matrix4<f32>,
}

impl Preview {
    pub fn new() -> Preview {
        static FALLBACK_VS_SRC: &'static str = "
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

        let program_error =
            Program::new(FALLBACK_VS_SRC.to_string(), FALLBACK_FS_SRC.to_string()).unwrap();

        Preview {
            program_valid: None,
            program_error,
            aabb: BoundingRect::new(Vector2::new(100.0, 000.0), Vector2::new(300.0, 300.0)),
            lookat: Matrix4::identity(),
        }
    }

    /// Sets the shader program that will be used to render a
    /// miniature preview window in the lower right-hand corner
    /// of the network.
    ///
    /// If `program` is `None`, then the renderer will use a
    /// fall-back shader to indicate the error state of the
    /// current graph.
    pub fn set_valid_program(&mut self, program: Option<Program>) {
        self.program_valid = program;
    }

    pub fn handle_interaction(&self, mouse: &MouseInfo) {
        // Sets lookat matrix based on mouse events
    }

    /// If a preview program has be assigned, render a miniature
    /// preview window in the lower right-hand corner of the
    /// network.
    pub fn draw(&self, renderer: &Renderer) {
        if let Some(ref program) = self.program_valid {
            renderer.draw_rect_with_program(&self.aabb, program);
        } else {
            renderer.draw_rect_with_program(&self.aabb, &self.program_error);
        }
    }
}
