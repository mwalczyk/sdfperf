use gl::{self, types::*};
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector2, Vector3, Vector4,
             Zero};

use bounds::Rect;
use color::Color;
use constants;
use interaction::{MouseInfo, Panel};
use program::Program;

use std::mem;
use std::ptr;
use std::os::raw::c_void;

#[derive(Copy, Clone)]
pub enum Shading {
    /// Display the z-depth of each fragment
    Depth,

    /// Display the number of steps taken along the ray
    Steps,

    /// Display the scene with ambient occlusion
    AmbientOcclusion,

    /// Display the normals of the underlying distance field
    Normals,

    /// Display the scene with diffuse lighting
    Diffuse,
}

struct VirtualCamera {
    /// The position of the camera
    position: Point3<f32>,

    /// The up vector of the camera
    up: Vector3<f32>,

    /// The direction that the camera is currently facing
    front: Vector3<f32>,

    /// The cross product of this camera's `up` and `front` vectors
    right: Vector3<f32>,

    /// The vertical angle of the camera
    pitch: f32,

    /// The horizontal angle of the camera
    yaw: f32,
}

impl VirtualCamera {
    fn new() -> VirtualCamera {
        VirtualCamera {
            position: Point3::new(0.0, 0.0, 5.0),
            up: Vector3::unit_y(),
            front: Vector3::new(0.0, 0.0, -1.0),
            right: Vector3::unit_x(),
            pitch: 0.0,
            yaw: -90.0,
        }
    }

    fn home(&mut self) {
        self.position = Point3::new(0.0, 0.0, 5.0);
        self.pitch = 0.0;
        self.yaw = -90.0;
    }

    fn rebuild_basis(&mut self) {
        self.front = Vector3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        ).normalize();

        self.right = self.front.cross(self.up).normalize()
    }
}

pub struct Preview {
    /// The valid shader program, if one exists
    program_valid: Option<Program>,

    /// The fallback program that will be used if `program_valid`
    /// is `None`
    program_error: Program,

    /// The bounding box of the preview window
    bounds: Rect,

    /// The virtual camera that will be used to view the scene
    camera: VirtualCamera,

    /// The current shading mode that will be applied to the scene
    shading: Shading,

    /// The OpenGL handle to the shader storage buffer object (SSBO)
    /// that will hold all of the op parameters
    ssbo: GLuint,
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
            const float tile = 15.0;
            vec2 uv = vs_texcoord * tile;
            vec2 ipos = floor(uv);

            float total = dot(ipos, vec2(1.0));
            float checkerboard = mod(total, 2.0);
            const float alpha = 0.25;

            o_color = vec4(vec3(checkerboard), alpha);
        }";

        let program_error =
            Program::new(FALLBACK_VS_SRC.to_string(), FALLBACK_FS_SRC.to_string()).unwrap();

        let mut ssbo = 0;
        unsafe {
            let ssbo_size = (constants::PARAMETER_SSBO_CAPACITY * mem::size_of::<Vector4<f32>>()) as GLsizeiptr;

            gl::CreateBuffers(1, &mut ssbo);
            gl::NamedBufferStorage(ssbo, ssbo_size, ptr::null(), gl::DYNAMIC_STORAGE_BIT);
        }
        Preview {
            program_valid: None,
            program_error,
            bounds: Rect::new(Vector2::new(400.0, 50.0), constants::PREVIEW_RESOLUTION),
            camera: VirtualCamera::new(),
            shading: Shading::Normals,
            ssbo,
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

    /// Writes `data` to the OpenGL buffer that this preview
    /// will use to populate shader parameters during rendering.
    pub fn update_params(&self, data: Vec<f32>) {
        unsafe {
            let data_size = (data.len() * mem::size_of::<f32>()) as GLsizeiptr;
            gl::NamedBufferSubData(self.ssbo, 0, data_size, data.as_ptr() as *const c_void);
        }
    }

    /// Sets the shading mode.
    pub fn set_shading(&mut self, shading: Shading) {
        self.shading = shading;
    }

    /// Homes the virtual preview camera.
    pub fn home(&mut self) {
        self.camera.home();
    }

    /// If a preview program has be assigned, render a miniature
    /// preview window in the lower right-hand corner of the
    /// network.
    pub fn prepare(&self, projection: &Matrix4<f32>) {
        if let Some(ref program) = self.program_valid {
            self.bind_transforms();
            program.bind();
            program.uniform_3f("u_camera_position", &self.camera.position.to_vec());
            program.uniform_3f("u_camera_front", &self.camera.front);
            program.uniform_1ui("u_shading", self.shading as u32);
            program.uniform_matrix_4f("u_model_matrix", &self.bounds.get_model_matrix());
            program.uniform_matrix_4f("u_projection_matrix", &projection);
        } else {
            self.program_error.bind();
            self.program_error
                .uniform_matrix_4f("u_model_matrix", &self.bounds.get_model_matrix());
            self.program_error
                .uniform_matrix_4f("u_projection_matrix", &projection);
        }
    }

    pub fn handle_interaction(&mut self, mouse: &MouseInfo) {
        if self.bounds.inside(&mouse.curr) {
            let offset = -mouse.velocity();

            // Handle camera rotation.
            if mouse.ldown {
                self.camera.yaw += offset.x * constants::PREVIEW_ROTATION_SENSITIVITY;
                self.camera.pitch += offset.y * constants::PREVIEW_ROTATION_SENSITIVITY;
                self.camera.pitch.min(89.0).max(-89.0);
                self.camera.rebuild_basis();
            }

            // Handle camera translation.
            if mouse.rdown {
                self.camera.position += self.camera.right * offset.x * constants::PREVIEW_TRANSLATION_SENSITIVITY;
                self.camera.position += self.camera.front * offset.y * constants::PREVIEW_TRANSLATION_SENSITIVITY;
            }
        }
    }

    fn bind_transforms(&self) {
        unsafe {
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, self.ssbo);
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.ssbo);
        }
    }
}
