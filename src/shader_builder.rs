use program::Program;
use graph::Graph;
use operator::Op;

use uuid::Uuid;

struct ShaderBuilder<'a> {
    shader_code: String,
    graph: &'a Graph
}

impl<'a> ShaderBuilder<'a> {

    pub fn new(graph: &'a Graph) -> ShaderBuilder<'a> {
        ShaderBuilder {
            shader_code: "".to_string(),
            graph
        }
    }

    // Compiles and links a new OpenGL shader program
    // with the same lifetime as this ShaderBuilder
    // instance
    pub fn build_program(&self) -> Program {
        static VS_SRC: &'static str = "";
        static FS_SRC: &'static str = "";

        // TODO: insert this ShaderBuilder's shader code into the fragment shader source template

        Program::new(VS_SRC.to_string(), self.shader_code.clone())
    }

    pub fn traverse(&self) -> Vec<Uuid> {
        let mut post_order_ids = Vec::new();

        // Is there an active render node in this graph?
        if let Some(root) = self.graph.root {

            // Since `root` is actually an `Option<Uuid>`, we get an actual
            // reference to the render op here.
            let render_op = self.graph.get_op(root).unwrap();

            for input_id in &render_op.input_connection_ids {
                post_order_ids.push(*input_id);
            }

            post_order_ids.push(root);
        }

        println!("Post-order traversal resulted in list:");
        println!("{:?}", post_order_ids);
        post_order_ids
    }

    fn recurse(&self, op: &Op) {

    }
}