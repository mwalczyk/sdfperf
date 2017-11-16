use program::Program;

use cgmath::{ Matrix, Matrix4, One, PerspectiveFov, Point2, Vector2, Point3, Vector3, Vector4 };

struct OperatorInfo {
    uuid: usize,
    name: String,
    // ...
}

trait Op {
    fn compute(&self) {

    }

    fn get_shader_code(&self) -> String {
        "test".to_string()
    }
}

// Render Root
//
// Generators:
// - Sphere
// - Cube
// - Plane
//
// Mergers:
// - Union
// - Intersect
// - Smooth minimum
//
// Deformers:
// - Noise
// - Sin
// - Cos
//
// Data:
// - Time
// - FFT
// - OSC
// - MIDI
#[derive(PartialEq)]
pub struct BoundingRect {
    pub upper_left: Vector2<f32>,
    pub size: Vector2<f32>
}

impl BoundingRect {
    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> BoundingRect {
        BoundingRect { upper_left, size }
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
        let translation = Matrix4::from_translation(
            Vector3::new(
                self.upper_left.x,
                self.upper_left.y,
                0.0)
        );

        let scale = Matrix4::from_nonuniform_scale(self.size.x, self.size.y, 0.0);

        translation * scale
    }
}

pub enum InteractionState {
    Unselected,
    Selected,
    ConnectSource,
    ConnectDestination
}

static REGION_SLOT_SIZE: f32 = 12.0;

pub struct Operator {
    pub input_connection_ids: Vec<usize>,
    pub output_connection_ids: Vec<usize>,
    pub region_operator: BoundingRect,
    pub region_slot_input: BoundingRect,
    pub region_slot_output: BoundingRect,
    pub state: InteractionState
}

impl Operator {

    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> Operator {
        Operator {
            input_connection_ids: vec![],
            output_connection_ids: vec![],
            region_operator: BoundingRect::new(upper_left, size),
            region_slot_input: BoundingRect::new(Vector2::new(upper_left.x - 6.0 * 0.5, upper_left.y + size.y * 0.5 - 6.0 * 0.5),
                                                 Vector2::new(6.0, 6.0)),
            region_slot_output: BoundingRect::new(Vector2::new(upper_left.x + size.x - 6.0 * 0.5, upper_left.y + size.y * 0.5 - 6.0 * 0.5),
                                                  Vector2::new(6.0, 6.0)),
            state: InteractionState::Selected
        }
    }

    pub fn set_screen_position(&mut self, position: &Vector2<f32>) {
        // ..
        // Rebuild the two bounding rectangles
    }
}

