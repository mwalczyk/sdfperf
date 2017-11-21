use program::Program;
use bounding_rect::BoundingRect;

use cgmath::{ Vector2, Vector3 };

static REGION_SLOT_SIZE: f32 = 6.0;

struct OpInfo {
    // A unique, numeric identifier - no two ops will have the same UUID
    pub uuid: usize,

    // The IDs of all ops that are connected to this op
    pub input_connection_ids: Vec<usize>,

    // The IDs of all ops that this op connects to
    pub output_connection_ids: Vec<usize>,

    // The user-defined name of this operator
    pub name: String,

    // The bounding box of the op
    pub region_operator: BoundingRect,

    // The bounding box of the op's input slot
    pub region_slot_input: BoundingRect,

    // The bounding box of the op's output slot
    pub region_slot_output: BoundingRect,

    // The current interaction state of the op
    pub state: InteractionState
}

enum OpType {
    // Generators
    Sphere,
    Box,
    Plane,

    // Combinations
    Union,
    Intersection,
    SmoothMinimum,

    // Final output node, required to render the graph
    Render
}

impl OpType {
    pub fn to_string(&self) -> String {
        match self {
            Sphere => "Sphere".to_string(),
            Box => "Box".to_string(),
            Plane => "Plane".to_string(),
            Union => "Union".to_string(),
            Intersection => "Intersection".to_string(),
            SmoothMinimum => "SmoothMinimum".to_string(),
            Render => "Render".to_string()
        }
    }

    pub fn get_input_capacity(&self) -> usize {
        match *self {
            OpType::Sphere | OpType::Box | OpType::Plane => 0,
            OpType::Union | OpType::Intersection | OpType::SmoothMinimum => 2,
            OpType::Render => 1
        }
    }

    pub fn get_unformatted_shader_code(&self) -> String {
        // In all branches, `p` refers to the current position along the ray,
        // i.e. the variable used in the `map` function
        match *self {
            OpType::Sphere => "float {} = sdfperf_sphere(p, {}, {});".to_string(),
            OpType::Box => "float {} = sdfperf_box(p, {}, {});".to_string(),
            OpType::Plane => "float {} = sdfperf_plane(p, {}, {});".to_string(),
            OpType::Union => "float {} = sdfperf_op_union({}, {})".to_string(),
            OpType::Intersection => "float {} = sdfperf_op_intersection({}, {})".to_string(),
            OpType::SmoothMinimum => "float {} = sdfperf_op_smin({}, {}, {})".to_string(),
            OpType::Render => "//render complete".to_string()
        }
    }
}

trait Op {
    fn compute(&self) -> Option<f32> {
        None
    }

    fn get_op_info(&self) -> &OpInfo;

    fn get_op_type(&self) -> OpType;

    fn get_formatted_shader_code(&self) -> String;

    // Returns the number of ops that are connected to this
    // op in the current graph
    fn get_number_of_active_inputs(&self) -> usize {
        self.get_op_info().input_connection_ids.len()
    }
}

pub enum InteractionState {
    Unselected,
    Selected,
    ConnectSource,
    ConnectDestination
}

//////////////////////////////////////////////////////////////////////////////////////////////////////
//
// Op Implementations
//
//////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct OpSphere {
    info: OpInfo,
    position: Vector3<f32>,
    radius: f32,
}

impl Op for OpSphere {
    fn get_op_info(&self) -> &OpInfo {
        &self.info
    }

    fn get_op_type(&self) -> OpType {
        OpType::Sphere
    }

    fn get_formatted_shader_code(&self) -> String {
        let mut code = self.get_op_type().get_unformatted_shader_code();
        let mut formatted = format!(code, self.name, self.position, self.radius);

        formatted
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////
//
//
//////////////////////////////////////////////////////////////////////////////////////////////////////








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

