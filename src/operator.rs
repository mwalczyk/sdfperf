use program::Program;
use bounding_rect::BoundingRect;

use cgmath::{ Vector2 };

struct OpInfo {
    // A unique, numeric identifier
    uuid: usize,

    // The maximum number of operators that can be connected to this operator
    input_capacity: usize,

    // The user-defined name of this operator
    name: String,
}

enum OpType {
    Generator,
    Combination,
    Data,
    Deformation,
    Render
}

trait Op {
    fn compute(&self) -> bool;

    fn get_op_info(&self) -> OpInfo;

    fn get_op_type(&self) -> OpType;
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
    pub region_slot_input: BoundingRect,
    pub region_slot_output: BoundingRect,
    pub state: InteractionState
}

impl Operator {

    pub fn new(upper_left: Vector2<f32>, size: Vector2<f32>) -> Operator {
        // The bounding region of the operator itself
        let region_operator = BoundingRect::new(upper_left, size);

        // The small bounding region of the input connection slot for this operator
        let region_slot_input = BoundingRect::new(Vector2::new(upper_left.x - REGION_SLOT_SIZE * 0.5, upper_left.y + size.y * 0.5 - REGION_SLOT_SIZE * 0.5),
                                                              Vector2::new(REGION_SLOT_SIZE, REGION_SLOT_SIZE));

        // The small bounding region of the output connection slot for this operator
        let region_slot_output = BoundingRect::new(Vector2::new(upper_left.x + size.x - REGION_SLOT_SIZE * 0.5, upper_left.y + size.y * 0.5 - REGION_SLOT_SIZE * 0.5),
                                                               Vector2::new(REGION_SLOT_SIZE, REGION_SLOT_SIZE));
        Operator {
            input_connection_ids: vec![],
            output_connection_ids: vec![],
            region_operator,
            region_slot_input,
            region_slot_output,
            state: InteractionState::Selected
        }
    }

    pub fn connect_to(&mut self, other: &mut Operator) {
        //self.output_connection_ids.push(other.id);
        //other.input_connection_ids.push(self.id);
    }

    pub fn set_screen_position(&mut self, position: &Vector2<f32>) {
        // ..
        // Rebuild the two bounding rectangles
    }
}

