use bounding_rect::BoundingRect;

use cgmath::Vector2;
use uuid::Uuid;

struct OpInfo {
    /// A unique, numeric identifier - no two ops will have the same UUID
    pub id: Uuid,

    /// The IDs of all ops that are connected to this op
    pub input_connection_ids: Vec<Uuid>,

    /// The IDs of all ops that this op connects to
    pub output_connection_ids: Vec<Uuid>,

    /// The user-defined name of this operator
    pub name: String,

    /// The bounding box of the op
    pub aabb_op: BoundingRect,

    /// The bounding box of the op's input slot
    pub aabb_slot_input: BoundingRect,

    /// The bounding box of the op's output slot
    pub aabb_slot_output: BoundingRect,

    /// The current interaction state of the op
    pub state: InteractionState
}

#[derive(PartialEq, Eq)]
pub enum OpType {
    /// Generates a sphere primitive
    Sphere,

    /// Generates a box primitive
    Box,

    /// Generates a plane primitive
    Plane,

    /// Merges two distance fields using a `min` operation
    Union,

    /// Merges two distance fields using a `max` operation
    Intersection,

    /// Merges two distance fields using a `smooth_min` operation
    SmoothMinimum,

    /// Final output node, required to render the graph
    Render,

    // TODO (materials): AmbientOcclusion, Normals, Phong, Constant
    // TODO (transforms): Scale, Translate, Rotate
    // TODO (repeaters): ModRepeat, ModRepeatCircular
    // TODO (displacers): PerlinNoise, FBMNoise
    // TODO (data): Sin, Cos, Time, Noise, Random
}

impl OpType {

    /// Converts the enum variant into a human-readable string format.
    pub fn to_string(&self) -> String {
        match *self {
            OpType::Sphere => "Sphere".to_string(),
            OpType::Box => "Box".to_string(),
            OpType::Plane => "Plane".to_string(),
            OpType::Union => "Union".to_string(),
            OpType::Intersection => "Intersection".to_string(),
            OpType::SmoothMinimum => "SmoothMinimum".to_string(),
            OpType::Render => "Render".to_string()
        }
    }

    /// Returns the maximum number of ops that can be connected to this
    /// op's input slot. Note that there is no equivalent `get_output_capacity`
    /// method, since an op's output slot can be connected to a potentially
    /// unbounded number of other ops.
    pub fn get_input_capacity(&self) -> usize {
        match *self {
            OpType::Sphere | OpType::Box | OpType::Plane => 0,
            OpType::Union | OpType::Intersection | OpType::SmoothMinimum => 2,
            OpType::Render => 1
        }
    }

    /// Returns `true` if this op's input slot can be connected to another
    /// op's output slot and `false` otherwise.
    pub fn has_inputs(&self) -> bool {
        self.get_input_capacity() > 0
    }

    /// Returns `true` if this op's output slot can be connected to another
    /// op's input slot and `false` otherwise.
    pub fn has_outputs(&self) -> bool {
        match *self {
            OpType::Render => false,
            _ => true
        }
    }

    pub fn get_unformatted_shader_code(&self) -> String {
        // In all branches, `p` refers to the current position along the ray,
        // i.e. the variable used in the `map` function.
        match *self {
            OpType::Sphere => "float sphere = sdf_sphere(p, vec3(0.0), 5.0);".to_string(),  //"float {} = sdf_sphere(p, {}, {});".to_string(),
            OpType::Box => "float {} = sdf_box(p, {}, {});".to_string(),
            OpType::Plane => "float {} = sdf_plane(p, {}, {});".to_string(),
            OpType::Union => "float {} = sdf_op_union({}, {});".to_string(),
            OpType::Intersection => "float {} = sdf_op_intersection({}, {});".to_string(),
            OpType::SmoothMinimum => "float {} = sdf_op_smin({}, {}, {});".to_string(),
            OpType::Render => "float render = sphere;".to_string()                          //"float {} = {};".to_string()
        }
    }
}

pub enum InteractionState {
    Unselected,
    Selected,
    ConnectSource,
    ConnectDestination
}

trait InterfaceElement {
    fn get_bounding_rect() -> BoundingRect;
}

pub struct Op {
    pub input_connection_ids: Vec<Uuid>,
    pub output_connection_ids: Vec<Uuid>,
    pub region_operator: BoundingRect,
    pub region_slot_input: BoundingRect,
    pub region_slot_output: BoundingRect,
    pub state: InteractionState,
    pub id: Uuid,
    pub op_type: OpType
}

impl Op {

    pub fn new(op_type: OpType, upper_left: Vector2<f32>, size: Vector2<f32>) -> Op {
        const SLOT_SIZE: Vector2<f32> = Vector2{ x: 6.0, y: 6.0 };

        // The bounding region of the op itself
        let region_operator = BoundingRect::new(upper_left, size);

        // The small bounding region of the input connection slot for this operator
        let region_slot_input = BoundingRect::new(Vector2::new(upper_left.x - SLOT_SIZE.x * 0.5, upper_left.y + size.y * 0.5 - SLOT_SIZE.y * 0.5),
                                                               SLOT_SIZE);

        // The small bounding region of the output connection slot for this operator
        let region_slot_output = BoundingRect::new(Vector2::new(upper_left.x + size.x - SLOT_SIZE.x * 0.5, upper_left.y + size.y * 0.5 - SLOT_SIZE.y * 0.5),
                                                                SLOT_SIZE);
        Op {
            input_connection_ids: Vec::new(),
            output_connection_ids: Vec::new(),
            region_operator,
            region_slot_input,
            region_slot_output,
            state: InteractionState::Selected,
            id: Uuid::new_v4(),
            op_type
        }
    }

    /// Returns the number of ops that are connected to this
    /// op in the current graph
    fn get_number_of_active_inputs(&self) -> usize {
        self.input_connection_ids.len()
    }

    pub fn connect_to(&mut self, other: &mut Op) -> bool {
        // Make sure that this op's output slot is active and the
        // other op's input slot isn't already at capacity.
        if self.op_type.has_outputs() && other.get_number_of_active_inputs() < other.op_type.get_input_capacity() {
            self.output_connection_ids.push(other.id);
            other.input_connection_ids.push(self.id);

            return true;
        }
        false
    }

    pub fn set_screen_position(&mut self, position: &Vector2<f32>) {
        // ..
        // Rebuild the two bounding rectangles - or translate them (create a bounding rectangle member function for this)
    }
}

