use bounding_rect::BoundingRect;
use graph::Connected;
use interaction::InteractionState;

use cgmath::{Vector2, Vector3, Vector4};
use uuid::Uuid;

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

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

    /// Merges two distance fields using a `max` operation and negation
    Subtraction,

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
    pub fn to_string(&self) -> &'static str {
        match *self {
            OpType::Sphere => "sphere",
            OpType::Box => "box",
            OpType::Plane => "plane",
            OpType::Union => "union",
            OpType::Subtraction => "subtraction",
            OpType::Intersection => "intersection",
            OpType::SmoothMinimum => "smooth_minimum",
            OpType::Render => "render",
        }
    }

    /// Returns the maximum number of ops that can be connected to this
    /// op's input slot. Note that there is no equivalent `get_output_capacity`
    /// method, since an op's output slot can be connected to a potentially
    /// unbounded number of other ops.
    pub fn get_input_capacity(&self) -> usize {
        match *self {
            OpType::Sphere | OpType::Box | OpType::Plane => 0,
            OpType::Union | OpType::Subtraction | OpType::Intersection | OpType::SmoothMinimum => 2,
            OpType::Render => 1,
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
            _ => true,
        }
    }

    pub fn get_code_template(&self) -> String {
        // In all branches, `p` refers to the current position along the ray,
        // i.e. the variable used in the `map` function.
        match *self {
            OpType::Sphere => "
                float s_NAME = transforms[INDEX].w;
                vec3 t_NAME = transforms[INDEX].xyz;
                float NAME = sdf_sphere(p / s_NAME + t_NAME, vec3(0.0), 1.0) * s_NAME;"
                .to_string(),
            OpType::Box => "
                float s_NAME = transforms[INDEX].w;
                vec3 t_NAME = transforms[INDEX].xyz;
                float NAME = sdf_box(p / s_NAME + t_NAME, vec3(1.0)) * s_NAME;"
                .to_string(),
            OpType::Plane => "
                float s_NAME = transforms[INDEX].w;
                vec3 t_NAME = transforms[INDEX].xyz;
                float NAME = sdf_plane(p / s_NAME + t_NAME, -1.0) * s_NAME;"
                .to_string(),
            OpType::Union => "float NAME = op_union(INPUT_A, INPUT_B);".to_string(),
            OpType::Subtraction => "float NAME = op_subtract(INPUT_A, INPUT_B);".to_string(),
            OpType::Intersection => "float NAME = op_intersect(INPUT_A, INPUT_B);".to_string(),
            OpType::SmoothMinimum => {
                "float NAME = op_smooth_min(INPUT_A, INPUT_B, 1.0);".to_string()
            }
            OpType::Render => "float NAME = INPUT_A;".to_string(),
        }
    }
}

pub enum Slot {
    Input(BoundingRect),
    Output(BoundingRect)
}

pub struct Op {
    /// The number of ops currently connected to this op
    pub active_inputs: usize,

    /// The bounding box of the op
    pub aabb_op: BoundingRect,

    /// The bounding box of the op's input slot
    pub aabb_slot_input: BoundingRect,

    /// The bounding box of the op's output slot
    pub aabb_slot_output: BoundingRect,

    /// The bounding box of the op's icon
    pub aabb_icon: BoundingRect,

    /// The current interaction state of the op
    pub state: InteractionState,

    /// A unique, numeric identifier - no two ops will have the same UUID
    pub uuid: Uuid,

    /// The name of the op (i.e. "sphere_0") as it will appear in the shader
    pub name: String,

    /// The op type
    pub family: OpType,

    /// The transform (translation and scale) that will be applied to the
    /// distance field represented by this op
    pub transform: Vector4<f32>,

    /// The index that will be used to grab this op's transform from the
    /// GPU-side buffer
    pub transform_index: usize,
}

impl Op {
    pub fn new(family: OpType, position: Vector2<f32>, size: Vector2<f32>) -> Op {
        const SLOT_SIZE: Vector2<f32> = Vector2 { x: 12.0, y: 12.0 };
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);

        // The bounding region of the op itself
        let aabb_op = BoundingRect::new(position, size);

        // The small bounding region of the input connection slot for this operator
        let aabb_slot_input = BoundingRect::new(
            Vector2::new(
                position.x - SLOT_SIZE.x * 0.5,
                position.y + size.y * 0.5 - SLOT_SIZE.y * 0.5,
            ),
            SLOT_SIZE,
        );

        // The small bounding region of the output connection slot for this operator
        let aabb_slot_output = BoundingRect::new(
            Vector2::new(
                position.x + size.x - SLOT_SIZE.x * 0.5,
                position.y + size.y * 0.5 - SLOT_SIZE.y * 0.5,
            ),
            SLOT_SIZE,
        );

        let aabb_icon = BoundingRect::new(position, Vector2::new(40.0, 40.0));

        let name = format!("{}_{}", family.to_string(), count);

        Op {
            active_inputs: 0,
            aabb_op,
            aabb_slot_input,
            aabb_slot_output,
            aabb_icon,
            state: InteractionState::Deselected,
            uuid: Uuid::new_v4(),
            name,
            family,
            transform: Vector4::new(0.0, 0.0, 0.0, 1.0),
            transform_index: count,
        }
    }

    /// Translates the op in the network editor by an amount
    /// `offset`. Internally, this translates each of the
    /// bounding rectangles that are owned by this op.
    pub fn translate(&mut self, offset: &Vector2<f32>) {
        self.aabb_op.translate(offset);
        self.aabb_slot_input.translate(offset);
        self.aabb_slot_output.translate(offset);
        self.aabb_icon.translate(offset);
    }

    pub fn set_transform(&mut self, transform: &Vector4<f32>) {
        self.transform = *transform;
    }
}

impl Connected for Op {
    fn has_inputs(&self) -> bool {
        self.family.has_inputs()
    }

    fn has_outputs(&self) -> bool {
        self.family.has_outputs()
    }

    fn get_number_of_available_inputs(&self) -> usize {
        self.family.get_input_capacity() - self.active_inputs
    }

    fn update_active_inputs_count(&mut self, count: usize) {
        self.active_inputs = count;
    }

    fn on_connect(&mut self) {
        self.active_inputs += 1;
    }

    fn on_disconnect(&mut self) {
        self.active_inputs -= 1;
    }
}
