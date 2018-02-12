use cgmath::{ Matrix, Matrix4, Vector2, Vector3 };

pub enum Edge {
    Left,
    Right,
    Top,
    Bottom
}

#[derive(PartialEq)]
pub struct BoundingRect {
    pub upper_left: Vector2<f32>,
    pub size: Vector2<f32>
}

impl BoundingRect {
    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> BoundingRect {
        BoundingRect { upper_left, size }
    }

    pub fn expand_from_center(&self, delta: &Vector2<f32>) -> BoundingRect {
        BoundingRect::new(self.upper_left - delta * 0.5, self.size + delta)
    }

    pub fn translate(&mut self, offset: &Vector2<f32>) {
        self.upper_left += *offset;
    }

    pub fn set_upper_left(&mut self, to: &Vector2<f32>) {
        self.upper_left = *to;
    }

    pub fn set_size(&mut self, to: &Vector2<f32>) {
        self.size = *to;
    }

    pub fn inside(&self, point: &Vector2<f32>) -> bool {
        if point.x > self.upper_left.x && point.x < (self.upper_left.x + self.size.x) &&
           point.y > self.upper_left.y && point.y < (self.upper_left.y + self.size.y) {
            return true;
        }
        false
    }

    pub fn inside_with_padding(&self, point: &Vector2<f32>, padding: f32) -> bool {
        if point.x > (self.upper_left.x - padding) && point.x < (self.upper_left.x + self.size.x + padding) &&
           point.y > (self.upper_left.y - padding) && point.y < (self.upper_left.y + self.size.y + padding) {
            return true;
        }
        false
    }

    pub fn centroid(&self) -> Vector2<f32> {
        Vector2::new(self.upper_left.x + self.size.x * 0.5,
                     self.upper_left.y + self.size.y * 0.5)
    }

    pub fn get_model_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::from_translation(Vector3::new(self.upper_left.x, self.upper_left.y, 0.0));
        let scale = Matrix4::from_nonuniform_scale(self.size.x, self.size.y, 0.0);

        translation * scale
    }
}