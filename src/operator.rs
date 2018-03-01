use bounds::{Edge, Rect};
use graph::Connected;
use interaction::InteractionState;

use cgmath::{Vector2, Vector3, Vector4, Zero};
use uuid::Uuid;

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub enum Connectivity {
    InputOutput,
    Input,
    Output,
}

#[derive(PartialEq, Eq)]
pub enum OpType {
    /// Generates a sphere primitive
    Sphere,

    /// Generates a box primitive
    Box,

    /// Generates a plane primitive
    Plane,

    /// Generates a torus primitive
    Torus,

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
            OpType::Torus => "torus",
            OpType::Union => "union",
            OpType::Subtraction => "subtraction",
            OpType::Intersection => "intersection",
            OpType::SmoothMinimum => "smooth_minimum",
            OpType::Render => "render",
        }
    }

    pub fn get_connectivity(&self) -> Connectivity {
        match *self {
            OpType::Sphere | OpType::Box | OpType::Plane | OpType::Torus => Connectivity::Output,
            OpType::Union | OpType::Subtraction | OpType::Intersection | OpType::SmoothMinimum => {
                Connectivity::InputOutput
            }
            OpType::Render => Connectivity::Input,
        }
    }

    /// Returns the maximum number of ops that can be connected to this
    /// op's input slot. Note that there is no equivalent `get_output_capacity`
    /// method, since an op's output slot can be connected to a potentially
    /// unbounded number of other ops.
    pub fn get_input_capacity(&self) -> usize {
        match *self {
            OpType::Sphere | OpType::Box | OpType::Plane | OpType::Torus => 0,
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
            OpType::Torus => "
                float s_NAME = transforms[INDEX].w;
                vec3 t_NAME = transforms[INDEX].xyz;
                float NAME = sdf_torus(p / s_NAME + t_NAME, vec2(1.0, 0.5)) * s_NAME;"
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
    Input(Rect),
    Output(Rect),
}

/// A struct representing a transformation that will be
/// applied to a distance field. Here, the xyz coordinates
/// of `data` represent a translation and the w-coordinate
/// represents a uniform scale.
pub struct Transform {
    pub data: Vector4<f32>,
    pub index: usize,
}

impl Transform {
    pub fn new(data: Vector4<f32>, index: usize) -> Transform {
        Transform { data, index }
    }

    pub fn translate(&mut self, val: &Vector3<f32>) {
        self.data.x += val.x;
        self.data.y += val.y;
        self.data.z += val.z;
    }

    pub fn scale(&mut self, val: f32) {
        self.data.w += val;
    }
}

pub struct Op {
    /// The number of ops currently connected to this op
    pub active_inputs: usize,

    /// The bounding box of the op
    pub bounds_body: Rect,

    /// The bounding box of the op's input slot
    pub bounds_input: Rect,

    /// The bounding box of the op's output slot
    pub bounds_output: Rect,

    /// The bounding box of the op's icon
    pub bounds_icon: Rect,

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
    pub transform: Transform,
}

impl Op {
    pub fn new(index: usize, family: OpType, position: Vector2<f32>, size: Vector2<f32>) -> Op {
        // Increment counter.
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);

        // Set up bounding boxes.
        let bounds_body = Rect::new(position, size);

        let mut bounds_input = Rect::square(Vector2::zero(), 12.0);
        bounds_input.center_on_edge(&bounds_body, Edge::Left);

        let mut bounds_output = Rect::square(Vector2::zero(), 12.0);
        bounds_output.center_on_edge(&bounds_body, Edge::Right);

        let mut bounds_icon = Rect::new(position, Vector2::new(40.0, 40.0));
        bounds_icon.translate(&Vector2::new(4.0, 4.0));

        let name = format!("{}_{}", family.to_string(), count);

        Op {
            active_inputs: 0,
            bounds_body,
            bounds_input,
            bounds_output,
            bounds_icon,
            state: InteractionState::Deselected,
            uuid: Uuid::new_v4(),
            name,
            family,
            transform: Transform::new(Vector4::new(0.0, 0.0, 0.0, 1.0), index),
        }
    }

    /// Translates the op in the network editor by an amount
    /// `offset`. Internally, this translates each of the
    /// bounding rectangles that are owned by this op.
    pub fn translate(&mut self, offset: &Vector2<f32>) {
        self.bounds_body.translate(offset);
        self.bounds_input.translate(offset);
        self.bounds_output.translate(offset);
        self.bounds_icon.translate(offset);
    }

    pub fn get_code(&self, input_a: Option<&str>, input_b: Option<&str>) -> String {
        let mut code = self.family.get_code_template();
        code = code.replace("NAME", &self.name);
        code = code.replace("INDEX", &self.transform.index.to_string());

        if let Some(a) = input_a {
            code = code.replace("INPUT_A", a);
        }
        if let Some(b) = input_b {
            code = code.replace("INPUT_B", b);
        }
        code
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
