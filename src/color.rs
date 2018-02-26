use std::ops::{Add, AddAssign};

use cgmath::Vector4;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    pub fn mono(rgb: f32, a: f32) -> Color {
        Color::new(rgb, rgb, rgb, a)
    }

    pub fn from_hex(code: u32, alpha: f32) -> Color {
        let r = ((code >> 16) & 0xFF) as f32 / 255.0;
        let g = ((code >> 8) & 0xFF) as f32 / 255.0;
        let b = ((code) & 0xFF) as f32 / 255.0;
        Color::new(r, g, b, alpha)
    }

    pub fn white() -> Color {
        Color::mono(1.0, 1.0)
    }

    pub fn black() -> Color {
        Color::mono(0.0, 1.0)
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, other: Color) -> Color {
        Color::new(
            self.r + other.r,
            self.g + other.g,
            self.b + other.b,
            self.a + other.a,
        )
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, other: Color) {
        *self = Color::new(
            self.r + other.r,
            self.g + other.g,
            self.b + other.b,
            self.a + other.a,
        );
    }
}

impl From<Vector4<f32>> for Color {
    fn from(item: Vector4<f32>) -> Self {
        Color::new(item.x, item.y, item.z, item.w)
    }
}

impl Into<Vector4<f32>> for Color {
    fn into(self) -> Vector4<f32> {
        Vector4::new(self.r, self.g, self.b, self.a)
    }
}

#[test]
fn test_white_hex() {
    assert_eq!(Color::from_hex(0xFFFFFF, 1.0), Color::white());
}

#[test]
fn test_black_hex() {
    assert_eq!(Color::from_hex(0x000000, 1.0), Color::black());
}
