use cgmath::{Matrix, Matrix4, SquareMatrix, Vector2, Vector3};

pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Copy, Clone, PartialEq)]
pub struct BoundingRect {
    upper_left: Vector2<f32>,
    size: Vector2<f32>,
    model_matrix: Matrix4<f32>,
}

impl BoundingRect {
    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> BoundingRect {
        let mut rect = BoundingRect {
            upper_left,
            size,
            model_matrix: Matrix4::identity(),
        };

        rect.rebuild_model_matrix();
        rect
    }

    pub fn get_upper_left(&self) -> &Vector2<f32> {
        &self.upper_left
    }

    pub fn get_size(&self) -> &Vector2<f32> {
        &self.size
    }

    pub fn snap_to_nearest(&mut self, increment: &Vector2<f32>) {
        // TODO
    }

    pub fn expand_from_center(&self, delta: &Vector2<f32>) -> BoundingRect {
        BoundingRect::new(self.upper_left - delta * 0.5, self.size + delta)
    }

    pub fn translate(&mut self, offset: &Vector2<f32>) {
        self.upper_left += *offset;
        self.rebuild_model_matrix();
    }

    pub fn set_upper_left(&mut self, to: &Vector2<f32>) {
        self.upper_left = *to;
        self.rebuild_model_matrix();
    }

    pub fn set_size(&mut self, to: &Vector2<f32>) {
        self.size = *to;
        self.rebuild_model_matrix();
    }

    pub fn inside(&self, point: &Vector2<f32>) -> bool {
        if point.x > self.upper_left.x && point.x < (self.upper_left.x + self.size.x)
            && point.y > self.upper_left.y && point.y < (self.upper_left.y + self.size.y)
        {
            return true;
        }
        false
    }

    pub fn inside_with_padding(&self, point: &Vector2<f32>, padding: f32) -> bool {
        if point.x > (self.upper_left.x - padding)
            && point.x < (self.upper_left.x + self.size.x + padding)
            && point.y > (self.upper_left.y - padding)
            && point.y < (self.upper_left.y + self.size.y + padding)
        {
            return true;
        }
        false
    }

    pub fn centroid(&self) -> Vector2<f32> {
        Vector2::new(
            self.upper_left.x + self.size.x * 0.5,
            self.upper_left.y + self.size.y * 0.5,
        )
    }

    pub fn get_model_matrix(&self) -> &Matrix4<f32> {
        &self.model_matrix
    }

    /// Caches a 4x4 model matrix that describes this bounding
    /// rectangle. This will be rebuilt any time the bounding
    /// rectangle changes size or location.
    fn rebuild_model_matrix(&mut self) {
        let translation =
            Matrix4::from_translation(Vector3::new(self.upper_left.x, self.upper_left.y, 0.0));
        let scale = Matrix4::from_nonuniform_scale(self.size.x, self.size.y, 0.0);

        self.model_matrix = translation * scale;
    }
}
