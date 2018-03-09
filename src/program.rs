use gl;
use gl::types::*;

use cgmath;
use cgmath::{Array, Matrix, Matrix4, Vector2, Vector3, Vector4};

use std::ptr;
use std::str;
use std::ffi::CString;
use std::collections::HashMap;

pub struct Uniform {
    location: i32,
    size: i32,
    // TODO this should be converted to a new type, like: https://github.com/glium/glium/blob/master/src/uniforms/value.rs
    ty: GLenum,
}

pub struct Program {
    pub id: GLuint,
    vs_src: String,
    fs_src: String,
    uniforms: HashMap<String, Uniform>,
}

impl Program {
    /// Compiles a shader of type `stage` from the source held in `src`.
    fn compile_shader(src: &String, stage: GLenum) -> Result<GLuint, String> {
        let shader;
        unsafe {
            shader = gl::CreateShader(stage);

            // Attempt to compile the shader.
            let c_str = CString::new(src.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);

            // Get the compile status.
            let mut status = gl::FALSE as GLint;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

            // Fail on error
            if status != (gl::TRUE as GLint) {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = Vec::with_capacity(len as usize);

                // Subtract 1 to skip the trailing null character.
                buffer.set_len((len as usize) - 1);

                gl::GetShaderInfoLog(
                    shader,
                    len,
                    ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );

                let error = String::from_utf8(buffer)
                    .ok()
                    .expect("ShaderInfoLog not valid utf8");
                return Err(error);
            }
        }

        Ok(shader)
    }

    fn link_program(vs: GLuint, fs: GLuint) -> Result<GLuint, String> {
        unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::LinkProgram(program);

            // Get the link status.
            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

            // If there was an error, return the error string.
            if status != (gl::TRUE as GLint) {
                let mut len: GLint = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = Vec::with_capacity(len as usize);

                // Subtract 1 to skip the trailing null character.
                buffer.set_len((len as usize) - 1);

                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );
                gl::DeleteShader(fs);
                gl::DeleteShader(vs);

                let error = String::from_utf8(buffer)
                    .ok()
                    .expect("ProgramInfoLog not valid utf8");
                return Err(error);
            }

            Ok(program)
        }
    }

    fn perform_reflection(&mut self) {
        unsafe {
            use std::mem;

            // Retrieve the number of active uniforms.
            let mut active_uniforms: GLint = mem::uninitialized();
            gl::GetProgramiv(self.id, gl::ACTIVE_UNIFORMS, &mut active_uniforms);

            // Retrieve the maximum length of each uniform name.
            let mut max_name_length: GLint = 0;
            gl::GetProgramiv(self.id, gl::ACTIVE_UNIFORM_MAX_LENGTH, &mut max_name_length);

            // Query for information about each uniform entry.
            for i in 0..active_uniforms {
                let mut name_bytes = Vec::with_capacity(max_name_length as usize);
                let mut name_length = 0;
                let mut size = 0;
                let mut ty = gl::NONE;

                gl::GetActiveUniform(
                    self.id,
                    i as GLuint,
                    max_name_length,
                    &mut name_length,
                    &mut size,
                    &mut ty,
                    name_bytes.as_mut_ptr() as *mut GLchar,
                );

                // Convert the byte array to a string.
                name_bytes.set_len(name_length as usize);
                let name = String::from_utf8(name_bytes).unwrap();

                // Finally, get the uniform's location.
                let location =
                    gl::GetUniformLocation(self.id, CString::new(name.clone()).unwrap().as_ptr());

                println!(
                    "Uniform Entry with name {:?}: size {}, type {}, location {}",
                    name, size, ty, location
                );
                self.uniforms.insert(name, Uniform { location, size, ty });
            }
        }
    }

    pub fn new(vs_src: String, fs_src: String) -> Option<Program> {
        // Make sure that compiling each of the shaders was successful.
        let compile_vs_res = Program::compile_shader(&vs_src, gl::VERTEX_SHADER);
        let compile_fs_res = Program::compile_shader(&fs_src, gl::FRAGMENT_SHADER);

        match (compile_vs_res, compile_fs_res) {
            (Ok(vs_id), Ok(fs_id)) => {
                // Make sure that linking the shader program was successful.
                if let Ok(id) = Program::link_program(vs_id, fs_id) {
                    // If everything went ok, return the shader program.
                    let mut valid_program = Program {
                        id,
                        vs_src,
                        fs_src,
                        uniforms: HashMap::new(),
                    };
                    valid_program.perform_reflection();

                    return Some(valid_program);
                } else {
                    return None;
                }
            }
            // Both shader stages resulted in an error.
            (Err(vs_err), Err(fs_err)) => {
                println!("{}", vs_err);
                println!("{}", fs_err);
                return None;
            }
            // The vertex shader resulted in an error.
            (Err(vs_err), Ok(_)) => {
                println!("{}", vs_err);
                return None;
            }
            // The fragment shader resulted in an error.
            (Ok(_), Err(fs_err)) => {
                println!("{}", fs_err);
                return None;
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::UseProgram(0);
        }
    }

    pub fn uniform_1i(&self, name: &str, value: i32) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform1i(self.id, location, value as gl::types::GLint);
        }
    }

    pub fn uniform_2i(&self, name: &str, value: &cgmath::Vector2<i32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform2iv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_3i(&self, name: &str, value: &cgmath::Vector3<i32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform3iv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_4i(&self, name: &str, value: &cgmath::Vector4<i32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform4iv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_1ui(&self, name: &str, value: u32) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform1ui(self.id, location, value as gl::types::GLuint);
        }
    }

    pub fn uniform_2ui(&self, name: &str, value: &cgmath::Vector2<u32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform2uiv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_3ui(&self, name: &str, value: &cgmath::Vector3<u32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform3uiv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_4ui(&self, name: &str, value: &cgmath::Vector4<u32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform4uiv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_1f(&self, name: &str, value: f32) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform1f(self.id, location, value as gl::types::GLfloat);
        }
    }

    pub fn uniform_2f(&self, name: &str, value: &cgmath::Vector2<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform2fv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_3f(&self, name: &str, value: &cgmath::Vector3<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform3fv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_4f(&self, name: &str, value: &cgmath::Vector4<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform4fv(self.id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_matrix_3f(&self, name: &str, value: &cgmath::Matrix3<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniformMatrix3fv(self.id, location, 1, gl::FALSE, value.as_ptr());
        }
    }

    pub fn uniform_matrix_4f(&self, name: &str, value: &cgmath::Matrix4<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniformMatrix4fv(self.id, location, 1, gl::FALSE, value.as_ptr());
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
