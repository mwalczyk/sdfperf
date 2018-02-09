use program::Program;
use graph::Graph;
use operator::Op;

use uuid::Uuid;

pub struct ShaderBuilder {
    shader_code: String,
}

impl ShaderBuilder {

    pub fn new() -> ShaderBuilder {
        ShaderBuilder {
            shader_code: "".to_string()
        }
    }

    pub fn traverse(&mut self, graph: &Graph) {
        let mut post_order_ids = Vec::new();

        // Is there an active render node in this graph?
        if let Some(root) = graph.root {

            // Since `root` is actually an `Option<Uuid>`, we get an actual
            // reference to the render op here.
            let render_op = graph.get_op(root).unwrap();

            // Recurse with each of the root op's inputs.
            for input_id in &render_op.input_connection_ids {
                post_order_ids.push(*input_id);
            }

            post_order_ids.push(root);
        }

        println!("Post-order traversal resulted in list:");
        println!("{:?}", post_order_ids);

        self.build_program(graph, post_order_ids);
    }

    fn recurse(&self, op: &Op) {
        // TODO
        // http://www.deepideas.net/deep-learning-from-scratch-i-computational-graphs/
    }

    pub fn build_program(&mut self, graph: &Graph, post_order_ids: Vec<Uuid>) {// -> Program {
        static VS_SRC: &'static str = "
        #version 430
        layout(location = 0) in vec2 position;
        uniform mat4 u_model_matrix;
        uniform mat4 u_projection_matrix;
        void main() {
            gl_Position = u_projection_matrix * u_model_matrix * vec4(position, 0.0, 1.0);
        }";

        static FS_SRC: &'static str = "";

        for uuid in post_order_ids {
            if let Some(op) = graph.get_op(uuid) {
                // Append this op's shader code.
                self.shader_code.push_str(&op.op_type.get_unformatted_shader_code()[..]);
            }
        }
        println!("Final shader code:");
        println!("{}", self.shader_code);

        // TODO: insert this ShaderBuilder's shader code into the fragment shader source template

        //Program::new(VS_SRC.to_string(), self.shader_code.clone())
    }
}