extern crate gl;

use gl::types::*;
use std::ptr;
use std::str;
use std::ffi::CString;

pub struct Program<'a> {
    pub program_id: GLuint,
    vertex_shader_src: &'a str,
    fragment_shader_src: &'a str
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

    pub fn new(vertex_shader_src: &'a str, fragment_shader_src: &'a str) -> Program<'a> {

        let vertex_shader_id = Program::compile_shader(vertex_shader_src, gl::VERTEX_SHADER);
        let fragment_shader_id = Program::compile_shader(fragment_shader_src, gl::FRAGMENT_SHADER);
        let program_id = Program::link_program(vertex_shader_id, fragment_shader_id);

        Program {
            program_id,
            vertex_shader_src,
            fragment_shader_src
        }
    }

    pub fn bind(&self) {
        unsafe { gl::UseProgram(self.program_id); }
    }

    pub fn unbind(&self) {
        unsafe { gl::UseProgram(0); }
    }

    pub fn uniform1f(&self, name: &str, value: f64) {
        unsafe {
            let location = gl::GetUniformLocation(self.program_id, CString::new(name).unwrap().as_ptr());
            gl::ProgramUniform1f(self.program_id, location, value as gl::types::GLfloat);
        }
    }
}

impl<'a> Drop for Program<'a> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program_id);
        }
    }
}
