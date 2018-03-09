use gl;
use gl::types::*;
use image::{self, GenericImage};
use cgmath::{self, Vector2};

use std::fs::File;
use std::path::Path;
use std::os::raw::c_void;

pub struct Texture {
    pixels: Vec<u8>,

    resolution: Vector2<f32>,

    id: GLuint,
}

impl Texture {
    pub fn new(path: &Path) -> Texture {
        let image = image::open(path).unwrap().to_rgba();
        let (w, h) = image.dimensions();
        let pixels: Vec<u8> = image.into_raw();

        let mut id = 0;
        unsafe {
            // Create the texture and set parameters.
            gl::CreateTextures(gl::TEXTURE_2D, 1, &mut id);
            gl::TextureParameteri(id, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as i32);
            gl::TextureParameteri(id, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TextureParameteri(id, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(id, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            // Allocate storage.
            gl::TextureStorage2D(id, 1, gl::RGBA8, w as i32, h as i32);
            gl::TextureSubImage2D(
                id,
                0,
                0,
                0,
                w as i32,
                h as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pixels.as_ptr() as *const c_void,
            );
        }

        let tex = Texture {
            pixels,
            resolution: Vector2::new(w as f32, h as f32),
            id,
        };
        tex.generate_mip_maps();
        tex
    }

    pub fn bind(&self, unit: u32) {
        unsafe {
            gl::BindTextureUnit(unit, self.id);
        }
    }

    pub fn unbind(&self, unit: u32) {
        unsafe {
            gl::BindTextureUnit(unit, 0);
        }
    }

    pub fn generate_mip_maps(&self) {
        unsafe {
            gl::GenerateTextureMipmap(self.id);
        }
    }
}
