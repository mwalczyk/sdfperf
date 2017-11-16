use gl;
use gl::types::*;

use cgmath;
use cgmath::{ Array, Matrix, Matrix4, Vector2, Vector3, Vector4 };

use std::ptr;
use std::str;
use std::ffi::CString;

pub struct UniformEntry {
    name: String,
    location: GLint
}

pub struct Program<'a> {
    pub program_id: GLuint,
    vert_shader_src: &'a str,
    frag_shader_src: &'a str
}

impl<'a> Program<'a> {

    fn compile_shader(src: &str, ty: GLenum) -> GLuint {
        let shader;

        unsafe {
            shader = gl::CreateShader(ty);

            // Attempt to compile the shader
            let c_str = CString::new(src.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);

            // Get the compile status
            let mut status = gl::FALSE as GLint;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

            // Fail on error
            if status != (gl::TRUE as GLint) {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = Vec::with_capacity(len as usize);
                buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    ptr::null_mut(),
                    buf.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "{}",
                    str::from_utf8(&buf)
                        .ok()
                        .expect("ShaderInfoLog not valid utf8")
                );
            }
        }

        shader
    }

    fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
        unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::LinkProgram(program);

            // Get the link status
            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

            // Fail on error
            if status != (gl::TRUE as GLint) {
                let mut len: GLint = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = Vec::with_capacity(len as usize);

                // Subtract 1 to skip the trailing null character
                buf.set_len((len as usize) - 1);
                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    buf.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "{}",
                    str::from_utf8(&buf)
                        .ok()
                        .expect("ProgramInfoLog not valid utf8")
                );

                gl::DeleteShader(fs);
                gl::DeleteShader(vs);
            }

            program
        }
    }

    fn perform_reflection(src: &str) {

    }

    pub fn new(vert_shader_src: &'a str, frag_shader_src: &'a str) -> Program<'a> {

        let vert_shader_id = Program::compile_shader(vert_shader_src, gl::VERTEX_SHADER);
        let frag_shader_id = Program::compile_shader(frag_shader_src, gl::FRAGMENT_SHADER);
        let program_id = Program::link_program(vert_shader_id, frag_shader_id);

        Program {
            program_id,
            vert_shader_src,
            frag_shader_src
        }
    }

    pub fn bind(&self) {
        unsafe { gl::UseProgram(self.program_id); }
    }

    pub fn unbind(&self) {
        unsafe { gl::UseProgram(0); }
    }

    pub fn uniform_1f(&self, name: &str, value: f32) {
        unsafe {
            let location = gl::GetUniformLocation(self.program_id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform1f(self.program_id, location, value as gl::types::GLfloat);
        }
    }

    pub fn uniform_2f(&self, name: &str, value: &cgmath::Vector2<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.program_id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform2fv(self.program_id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_3f(&self, name: &str, value: &cgmath::Vector3<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.program_id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform3fv(self.program_id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_4f(&self, name: &str, value: &cgmath::Vector4<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.program_id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform4fv(self.program_id, location, 1, value.as_ptr());
        }
    }

    pub fn uniform_matrix_4f(&self, name: &str, value: &cgmath::Matrix4<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.program_id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniformMatrix4fv(self.program_id, location, 1, gl::FALSE, value.as_ptr());
        }
    }
}

impl<'a> Drop for Program<'a> {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.program_id); }
    }
}
