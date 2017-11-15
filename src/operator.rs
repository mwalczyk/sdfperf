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
}

pub enum InteractionState {
    Unselected,
    Selected,
    ConnectSource,
    ConnectDestination
}

static REGION_SLOT_SIZE: f32 = 6.0;

pub struct Operator {
    pub input_connection_ids: Vec<usize>,
    pub output_connection_ids: Vec<usize>,
    pub region_operator: BoundingRect,
    pub region_slot: BoundingRect,
    pub state: InteractionState
}

impl Operator {

    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> Operator {
        Operator {
            input_connection_ids: vec![],
            output_connection_ids: vec![],
            region_operator: BoundingRect::new(upper_left, size),
            region_slot: BoundingRect::new(Vector2::new(upper_left.x + size.x - 6.0 * 0.5, upper_left.y + size.y * 0.5 - 6.0 * 0.5),
                                           Vector2::new(6.0, 6.0)),
            state: InteractionState::Selected
        }
    }

    pub fn set_screen_position(&mut self, position: &Vector2<f32>) {
        // ..
        // Rebuild the two bounding rectangles
    }
}

