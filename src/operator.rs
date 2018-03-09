use bounds::{Edge, Rect};
use graph::Connected;
use interaction::InteractionState;
use renderer::{DrawParams, Drawable};

use cgmath::{Vector2, Vector3, Vector4, Zero};
use uuid::Uuid;

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub enum Connectivity {
    InputOutput,
    Input,
    Output,
}

pub enum ConnectionType {
    /// A connection between two ops that are compatible and from
    /// the same family
    Direct,

    /// A connection between two ops that are compatible but from
    /// different families
    Indirect,

    /// An invalid connection
    Invalid,
}

const PARAMETER_CAPACITY: usize = 4;

#[derive(Copy, Clone, PartialEq)]
pub struct Parameters {
    /// The actual parameter data
    data: [f32; PARAMETER_CAPACITY],

    /// The names of each component of this parameter
    names: [&'static str; PARAMETER_CAPACITY],

    /// The index of this parameter in the SSBO that will hold
    /// all of the op parameters at runtime
    index: usize,

    /// The minimum value of each component of this parameter -
    /// in other words, `data[0]` should always be greater than
    /// or equal to `min[0]`
    min: [f32; PARAMETER_CAPACITY],

    /// The maximum value of each component of this parameter -
    /// in other words, `data[0]` should always be less than
    /// or equal to `max[0]`
    max: [f32; PARAMETER_CAPACITY],

    /// The step size that will be taken when a component of
    /// this parameter is incremented or decremented
    step: [f32; PARAMETER_CAPACITY],
}

impl Parameters {
    pub fn new(
        data: [f32; PARAMETER_CAPACITY],
        names: [&'static str; PARAMETER_CAPACITY],
        index: usize,
        min: [f32; PARAMETER_CAPACITY],
        max: [f32; PARAMETER_CAPACITY],
        step: [f32; PARAMETER_CAPACITY],
    ) -> Parameters {
        Parameters {
            data,
            names,
            index,
            min,
            max,
            step,
        }
    }

    pub fn get_data(&self) -> &[f32; PARAMETER_CAPACITY] {
        &self.data
    }

    pub fn get_data_mut(&mut self) -> &mut [f32; PARAMETER_CAPACITY] {
        &mut self.data
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn set_data(&mut self, values: [f32; PARAMETER_CAPACITY]) {
        for (i, v) in values.iter().enumerate() {
            self.data[i] += v;
        }
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Parameters::new(
            [0.0; PARAMETER_CAPACITY],
            ["param0", "param1", "param2", "param3"],
            0,
            [0.0; PARAMETER_CAPACITY],
            [0.0; PARAMETER_CAPACITY],
            [0.0; PARAMETER_CAPACITY],
        )
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum DomainType {
    Root,
    Transform,
    Twist,
    Bend,
}

#[derive(Copy, Clone, PartialEq)]
pub enum DataType {
    Time,
    Math,
    Sin,
    Cos,
    Noise,
    Mouse,
    Audio,
}

#[derive(Copy, Clone, PartialEq)]
pub enum PrimitiveType {
    Sphere,
    Box,
    Plane,
    Torus,
    Union,
    Subtraction,
    Intersection,
    SmoothMinimum,
    Render,
}

#[derive(Copy, Clone, PartialEq)]
pub enum DisplacementType {
    Noise,
    Sin,
    Cos,
}

#[derive(Copy, Clone, PartialEq)]
pub enum OpFamily {
    // TODO: Data,
    // TODO: Displacement,
    Domain(DomainType),
    Primitive(PrimitiveType),
}

impl OpFamily {
    /// Converts the nested enum variant into a human-readable string format.
    pub fn to_string(&self) -> &'static str {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Root => "root",
                DomainType::Transform => "transform",
                DomainType::Twist => "twist",
                DomainType::Bend => "bend",
            },
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::Sphere => "sphere",
                PrimitiveType::Box => "box",
                PrimitiveType::Plane => "plane",
                PrimitiveType::Torus => "torus",
                PrimitiveType::Union => "union",
                PrimitiveType::Subtraction => "subtraction",
                PrimitiveType::Intersection => "intersection",
                PrimitiveType::SmoothMinimum => "smooth_minimum",
                PrimitiveType::Render => "render",
            },
        }
    }

    /// Returns an enum that describes the connectivity of this op family
    /// (whether it accepts inputs, outputs, or both).
    pub fn get_connectivity(&self) -> Connectivity {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Root => Connectivity::Output,
                _ => Connectivity::InputOutput,
            },
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::Render => Connectivity::Input,
                _ => Connectivity::InputOutput,
            },
        }
    }

    /// Returns the maximum number of ops that can be connected to this
    /// op's input slot. Note that there is no equivalent `get_output_capacity`
    /// method, since an op's output slot can be connected to a potentially
    /// unbounded number of other ops.
    pub fn get_input_capacity(&self) -> usize {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Root => 0,
                _ => 1,
            },
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::Union
                | PrimitiveType::Subtraction
                | PrimitiveType::Intersection
                | PrimitiveType::SmoothMinimum => 2,
                _ => 1,
            },
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
            OpFamily::Domain(domain) => true,
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::Render => false,
                _ => true,
            },
        }
    }

    /// Returns a formattable string of shader code that corresponds to
    /// this op family.
    pub fn get_code_template(&self) -> String {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Root => "
                    vec3 p_NAME = p;
                    float s_NAME = 1.0;"
                    .to_string(),
                DomainType::Transform => "
                    float s_NAME = params[INDEX].w * s_INPUT_A;
                    vec3 t_NAME = params[INDEX].xyz;
                    vec3 p_NAME = p_INPUT_A / s_NAME + t_NAME;"
                    .to_string(),
                DomainType::Twist => "
                    float s_NAME = s_INPUT_A;
                    vec3 p_NAME = domain_twist(p_INPUT_A, params[INDEX].x);"
                    .to_string(),
                DomainType::Bend => "
                    float s_NAME = s_INPUT_A;
                    vec3 p_NAME = domain_bend(p_INPUT_A, params[INDEX].x);"
                    .to_string(),
            },
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::Sphere => {
                    "float NAME = sdf_sphere(p_INPUT_A, vec3(0.0), 1.0) * s_INPUT_A;".to_string()
                }
                PrimitiveType::Box => {
                    "float NAME = sdf_box(p_INPUT_A, vec3(1.0)) * s_INPUT_A;".to_string()
                }
                PrimitiveType::Plane => {
                    "float NAME = sdf_plane(p_INPUT_A, -1.0) * s_INPUT_A;".to_string()
                }
                PrimitiveType::Torus => {
                    "float NAME = sdf_torus(p_INPUT_A, vec2(1.0, 0.5)) * s_INPUT_A;".to_string()
                }
                PrimitiveType::Union => "float NAME = op_union(INPUT_A, INPUT_B);".to_string(),
                PrimitiveType::Subtraction => {
                    "float NAME = op_subtract(INPUT_A, INPUT_B);".to_string()
                }
                PrimitiveType::Intersection => {
                    "float NAME = op_intersect(INPUT_A, INPUT_B);".to_string()
                }
                PrimitiveType::SmoothMinimum => {
                    "float NAME = op_smooth_min(INPUT_A, INPUT_B, params[INDEX].x);".to_string()
                }
                PrimitiveType::Render => "float NAME = INPUT_A;".to_string(),
            },
        }
    }

    /// Returns `true` if this op family can connect to `other`, either
    /// directly or indirectly.
    pub fn can_connect_to(&self, other: OpFamily) -> bool {
        match *self {
            // This operator is a domain operator.
            OpFamily::Domain(domain) => match other {
                OpFamily::Domain(other_domain) => return true,
                OpFamily::Primitive(other_primitive) => match other_primitive {
                    PrimitiveType::Sphere
                    | PrimitiveType::Box
                    | PrimitiveType::Plane
                    | PrimitiveType::Torus => return true,
                    _ => return false,
                },
            },
            // This operator is a primitive operator.
            OpFamily::Primitive(primitive) => match other {
                OpFamily::Domain(other_domain) => return false,
                OpFamily::Primitive(other_primitive) => return true,
            },
        }
    }

    /// Returns the connection type between this op family and `other`. A
    /// connection can be either direct, indirect, or invalid.
    pub fn get_connection_type(&self, other: OpFamily) -> ConnectionType {
        match *self {
            // This operator is a domain operator.
            OpFamily::Domain(domain) => match other {
                OpFamily::Domain(other_domain) => ConnectionType::Direct,
                OpFamily::Primitive(other_primitive) => ConnectionType::Indirect,
            },
            // This operator is a primitive operator.
            OpFamily::Primitive(primitive) => match other {
                OpFamily::Domain(other_domain) => ConnectionType::Invalid,
                OpFamily::Primitive(other_primitive) => ConnectionType::Direct,
            },
        }
    }

    /// Returns the default parameters for this op family.
    pub fn get_default_params(&self) -> Parameters {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Transform => Parameters::new(
                    [0.0, 0.0, 0.0, 1.0],
                    ["translate_x", "translate_y", "translate_z", "scale"],
                    0,
                    [-10.0, -10.0, -10.0, 0.1],
                    [10.0, 10.0, 10.0, 10.0],
                    [0.5, 0.5, 0.5, 0.1],
                ),
                DomainType::Twist => Parameters::new(
                    [4.0, 4.0, 0.0, 0.0],
                    ["twist_x", "twist_y", "", ""],
                    0,
                    [0.0, 0.0, 0.0, 0.0],
                    [20.0, 20.0, 0.0, 0.0],
                    [1.0, 1.0, 0.0, 0.0],
                ),
                DomainType::Bend => Parameters::new(
                    [0.5, 0.5, 0.0, 0.0],
                    ["bend_x", "bend_y", "", ""],
                    0,
                    [0.0, 0.0, 0.0, 0.0],
                    [2.0, 2.0, 0.0, 0.0],
                    [0.05, 0.05, 0.0, 0.0],
                ),
                _ => Parameters::default(),
            },
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::SmoothMinimum => Parameters::new(
                    [1.0, 0.0, 0.0, 0.0],
                    ["exponent", "", "", ""],
                    0,
                    [0.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [0.1, 0.0, 0.0, 0.0],
                ),
                _ => Parameters::default(),
            },
        }
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

    /// The op family
    pub family: OpFamily,

    /// This op's parameters, which may or may not be used by the shader
    pub params: Parameters,
}

impl Op {
    pub fn new(family: OpFamily, position: Vector2<f32>, size: Vector2<f32>) -> Op {
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
            params: family.get_default_params(),
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

        code = code.replace("INDEX", &self.params.index.to_string());

        if let Some(a) = input_a {
            code = code.replace("INPUT_A", a);
        }
        if let Some(b) = input_b {
            code = code.replace("INPUT_B", b);
        }
        code
    }

    pub fn get_params(&self) -> &Parameters {
        &self.params
    }

    pub fn get_params_mut(&mut self) -> &mut Parameters {
        &mut self.params
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

impl<'a> Drawable<'a> for Op {
    fn get_draw_params(&'a self) -> DrawParams<'a> {
        DrawParams::Rectangle(&self.bounds_body)
    }
}
