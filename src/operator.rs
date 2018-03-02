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

/// A struct representing a transformation that will be
/// applied to a distance field. Here, the xyz coordinates
/// of `data` represent a translation and the w-coordinate
/// represents a uniform scale.
#[derive(Copy, Clone, PartialEq)]
pub struct Parameters {
    pub data: Vector4<f32>,
    pub index: usize,
    pub min: Vector4<f32>,
    pub max: Vector4<f32>,
    pub step: Vector4<f32>,
}

impl Parameters {
    pub fn new(
        data: Vector4<f32>,
        index: usize,
        min: Vector4<f32>,
        max: Vector4<f32>,
        step: Vector4<f32>,
    ) -> Parameters {
        Parameters {
            data,
            index,
            min,
            max,
            step,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum DomainType {
    Root,
    Transform(Parameters),
    Twist(Parameters),
    //Bend
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
    SmoothMinimum(Parameters),
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
    //Data,
    //Displacement,
    Domain(DomainType),
    Primitive(PrimitiveType),
    // INPUT_A += noise(p_INPUT_A);
}

impl OpFamily {
    /// Converts the nested enum variant into a human-readable string format.
    pub fn to_string(&self) -> &'static str {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Root => "root",
                DomainType::Transform(_) => "transform",
                DomainType::Twist(_) => "twist",
            },
            OpFamily::Primitive(primitive) => match primitive {
                PrimitiveType::Sphere => "sphere",
                PrimitiveType::Box => "box",
                PrimitiveType::Plane => "plane",
                PrimitiveType::Torus => "torus",
                PrimitiveType::Union => "union",
                PrimitiveType::Subtraction => "subtraction",
                PrimitiveType::Intersection => "intersection",
                PrimitiveType::SmoothMinimum(_) => "smooth_minimum",
                PrimitiveType::Render => "render",
            },
        }
    }

    pub fn get_params(&self) -> Option<&Parameters> {
        match *self {
            OpFamily::Domain(ref domain) => match *domain {
                DomainType::Transform(ref params) | DomainType::Twist(ref params) => {
                    return Some(params)
                }
                _ => None,
            },
            OpFamily::Primitive(ref primitive) => match *primitive {
                PrimitiveType::SmoothMinimum(ref params) => return Some(params),
                _ => None,
            },
        }
    }

    pub fn get_params_mut(&mut self) -> Option<&mut Parameters> {
        match *self {
            OpFamily::Domain(ref mut domain) => match *domain {
                DomainType::Transform(ref mut params) => return Some(params),
                DomainType::Twist(ref mut params) => return Some(params),
                _ => None,
            },
            OpFamily::Primitive(ref mut primitive) => match *primitive {
                PrimitiveType::SmoothMinimum(ref mut params) => return Some(params),
                _ => None,
            },
        }
    }

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
                | PrimitiveType::SmoothMinimum(_) => 2,
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

    pub fn get_code_template(&self) -> String {
        match *self {
            OpFamily::Domain(domain) => match domain {
                DomainType::Root => "
                    vec3 p_NAME = p;
                    float s_NAME = 1.0;"
                    .to_string(),
                DomainType::Transform(_) => "
                    float s_NAME = params[INDEX].w * s_INPUT_A;
                    vec3 t_NAME = params[INDEX].xyz;
                    vec3 p_NAME = p_INPUT_A / s_NAME + t_NAME;"
                    .to_string(),
                DomainType::Twist(_) => "
                    float s_NAME = s_INPUT_A;
                    vec3 p_NAME = domain_twist(p_INPUT_A, params[INDEX].x);"
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
                PrimitiveType::SmoothMinimum(_) => {
                    "float NAME = op_smooth_min(INPUT_A, INPUT_B, params[INDEX].x);".to_string()
                }
                PrimitiveType::Render => "float NAME = INPUT_A;".to_string(),
            },
        }
    }

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

        // Populate params.
        if let Some(ref params) = self.family.get_params() {
            code = code.replace("INDEX", &params.index.to_string());
        }

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
