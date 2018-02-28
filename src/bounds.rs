use cgmath::{Matrix, Matrix4, SquareMatrix, Vector2, Vector3};

pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Copy, Clone, PartialEq)]
pub struct Rect {
    upper_left: Vector2<f32>,
    size: Vector2<f32>,
    model_matrix: Matrix4<f32>,
}

impl Rect {
    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> Rect {
        let mut rect = Rect {
            upper_left,
            size,
            model_matrix: Matrix4::identity(),
        };

        rect.rebuild_model_matrix();
        rect
    }

    pub fn expanded_from(other: &Rect, amount: &Vector2<f32>) -> Rect {
        Rect::new(other.upper_left - amount * 0.5, other.size + amount)
    }

    pub fn square(upper_left: Vector2<f32>, size: f32) -> Rect {
        Rect::new(upper_left, Vector2::new(size, size))
    }

    /// Returns the upper-left corner of the rectangle.
    pub fn get_upper_left(&self) -> &Vector2<f32> {
        &self.upper_left
    }

    /// Returns the size (width and height) of the rectangle.
    pub fn get_size(&self) -> &Vector2<f32> {
        &self.size
    }

    /// Returns the model matrix represented by this rectangle.
    pub fn get_model_matrix(&self) -> &Matrix4<f32> {
        &self.model_matrix
    }

    /// Sets the upper-left corner of the rectangle.
    pub fn set_upper_left(&mut self, to: &Vector2<f32>) {
        self.upper_left = *to;
        self.rebuild_model_matrix();
    }

    /// Sets the size (width and height) of the rectangle.
    pub fn set_size(&mut self, to: &Vector2<f32>) {
        self.size = *to;
        self.rebuild_model_matrix();
    }

    pub fn snap_to_nearest(&mut self, increment: &Vector2<f32>) {
        // TODO
    }

    pub fn expand_from_center(&self, delta: &Vector2<f32>) -> Rect {
        Rect::new(self.upper_left - delta * 0.5, self.size + delta)
    }

    pub fn center_on_edge(&mut self, other: &Rect, edge: Edge) {
        let center = other.midpoint(edge) - self.get_size() * 0.5;
        self.set_upper_left(&center);
    }

    pub fn translate(&mut self, offset: &Vector2<f32>) {
        self.upper_left += *offset;
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

    pub fn midpoint(&self, edge: Edge) -> Vector2<f32> {
        match edge {
            Edge::Left => Vector2::new(self.upper_left.x, self.upper_left.y + self.size.y * 0.5),
            Edge::Right => Vector2::new(
                self.upper_left.x + self.size.x,
                self.upper_left.y + self.size.y * 0.5,
            ),
            Edge::Top => Vector2::new(self.upper_left.x + self.size.x * 0.5, self.upper_left.y),
            Edge::Bottom => Vector2::new(
                self.upper_left.x + self.size.x * 0.5,
                self.upper_left.y + self.size.y,
            ),
        }
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

impl Default for Rect {
    fn default() -> Self {
        Rect {
            upper_left: Vector2::new(0.0, 0.0),
            size: Vector2::new(1.0, 1.0),
            model_matrix: Matrix4::identity(),
        }
    }
}
