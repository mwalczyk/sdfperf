use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector2, Vector3, Zero};

use bounding_rect::BoundingRect;
use color::Color;
use interaction::MouseInfo;
use program::Program;
use renderer::Renderer;

#[derive(Copy, Clone)]
pub enum Shading {
    // TODO: eventually, these could be structs with memebers like `color`
    Diffuse,
    Constant,
    Normals,
}

pub struct Preview {
    program_valid: Option<Program>,

    program_error: Program,

    aabb: BoundingRect,

    look_at: Matrix4<f32>,

    camera_position: Point3<f32>,

    yaw: f32,

    pitch: f32,

    shading: Shading,
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
            look_at: Matrix4::look_at(
                Point3::new(0.0, 0.0, 3.0),
                Point3::new(0.0, 0.0, 0.0),
                Vector3::unit_y(),
            ),
            camera_position: Point3::new(0.0, 0.0, 3.0),
            yaw: -90.0,
            pitch: 0.0,
            shading: Shading::Normals,
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

    /// Sets the SDF shading mode.
    pub fn set_shading(&mut self, shading: Shading) {
        self.shading = shading;
    }

    /// Homes the virtual preview camera.
    pub fn home(&mut self) {
        self.yaw = -90.0;
        self.pitch = 0.0;
        self.camera_position = Point3::new(0.0, 0.0, 3.0);
    }

    pub fn handle_interaction(&mut self, mouse: &MouseInfo) {
        // Rebuilds the look-at matrix based on mouse events.
        if self.aabb.inside(&mouse.curr) {
            let mut front = Vector3::zero();
            let offset = mouse.curr - mouse.last;
            const ROTATION_SENSITIVITY: f32 = 0.25;
            const TRANSLATION_SENSITIVITY: f32 = 0.005;

            // Handle camera rotation.
            if mouse.ldown {
                self.yaw += offset.x * ROTATION_SENSITIVITY;
                self.pitch += offset.y * ROTATION_SENSITIVITY;

                // Prevent the screen from flipping.
                self.pitch.min(89.0).max(-89.0);
            }

            // Based on the Euler angles calculated above,
            // create the virtual camera's "front" (forward-facing)
            // vector.
            front.x = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
            front.y = self.pitch.to_radians().sin();
            front.z = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();
            front.normalize();

            // TODO: this isn't working.
            // Handle camera translation.
            if mouse.rdown {
                // Strafe left and right.
                self.camera_position +=
                    front.cross(Vector3::unit_y()).normalize() * offset.x * TRANSLATION_SENSITIVITY;

                // Move forward and backwards.
                self.camera_position += front * offset.y * TRANSLATION_SENSITIVITY;
            }

            self.look_at = Matrix4::look_at(
                self.camera_position,
                self.camera_position + front,
                Vector3::unit_y(),
            );
        }
    }

    /// If a preview program has be assigned, render a miniature
    /// preview window in the lower right-hand corner of the
    /// network.
    pub fn draw(&self, renderer: &Renderer) {
        if let Some(ref program) = self.program_valid {
            // Set the look-at matrix that will be used to construct
            // the virtual camera.
            program.uniform_matrix_4f("u_look_at_matrix", &self.look_at);
            program.uniform_3f("u_camera_position", &self.camera_position.to_vec());
            program.uniform_1ui("u_shading", self.shading as u32);
            renderer.draw_rect_with_program(&self.aabb, program);
        } else {
            renderer.draw_rect_with_program(&self.aabb, &self.program_error);
        }
    }
}