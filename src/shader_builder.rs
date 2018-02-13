use graph::Graph;
use operator::{Op, OpType};
use program::Program;
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

    /// Performs a post-order traversal of the ops in `graph`,
    /// returning the shader program that is described by the
    /// current network.
    pub fn traverse(&mut self, graph: &Graph) -> Option<Program> {
        let mut uuids = Vec::new();

        // Is there an active render node in this graph?
        if let Some(root_uuid) = graph.root {
            let root = graph.get_op(root_uuid).unwrap();

            // Traverse the graph, starting at the root op.
            self.recurse(graph, root, &mut uuids);
        }

        let (vs_src, fs_src) = self.build_sources(graph, uuids);

        Program::new(vs_src, fs_src)
    }

    /// Examine a `root` op's inputs and recurse backwards until
    /// reaching a leaf node (i.e. an op with no other inputs).
    fn recurse(&self, graph: &Graph, root: &Op, uuids: &mut Vec<Uuid>) {
        if root.op_type.has_inputs() {
            for input_id in &root.input_uuids {
                self.recurse(graph, graph.get_op(*input_id).unwrap(), uuids);
            }
        }

        // Finally, push back the root op's UUID.
        uuids.push(root.id);
    }

    /// Given a list of op UUIDs in the proper post-order, builds
    /// and returns the appropriate shader code.
    fn build_sources(&mut self, graph: &Graph, uuids: Vec<Uuid>) -> (String, String) {

        // TODO: each op will need something like this as part of its shader code
        static TRANSFORMS: &str = "
        struct transform
        {
            vec3 r; // rotation
            vec3 s; // scale
            vec3 t; // translation
        }

        uint sphere_id = 0;
        vec3 r = ubo[sphere_id].r;
        vec3 s = ubo[sphere_id].s;
        vec3 t = ubo[sphere_id].t;

        // do some stuff to transform `p`
        float node = ...;
        float sphere = sdf_sphere(node, vec3(0.0), 1.0);
        ";

        static HEADER: &str = "
        #version 430
        layout (location = 0) in vec2 vs_texcoord;
        layout (location = 0) out vec4 o_color;

        const uint MAX_STEPS = 128u;
        const float MAX_TRACE_DISTANCE = 32.0;
        const float MIN_HIT_DISTANCE = 0.0001;

        struct ray
        {
            vec3 o;
            vec3 d;
        };

        mat3 lookat(in vec3 t, in vec3 p)
        {
            vec3 k = normalize(t - p);
            vec3 i = cross(k, vec3(0.0, 1.0, 0.0));
            vec3 j = cross(i, k);

            return mat3(i, j, k);
        }

        float op_union(float a, float b)
        {
            return min(a, b);
        }

        float op_intersect(float a, float b)
        {
            return max(a, b);
        }

        float op_smooth_min(float a, float b, float k)
        {
            float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
            return mix(b, a, h) - k * h * (1.0 - h);
        }

        float sdf_sphere(in vec3 p, in vec3 center, float radius)
        {
            return length(center - p) - radius;
        }

        float sdf_box(in vec3 p, in vec3 b)
        {
          vec3 d = abs(p) - b;
          return min(max(d.x, max(d.y, d.z)), 0.0) + length(max(d, 0.0));
        }

        float sdf_plane(in vec3 p, in float h)
        {
            return p.y - h;
        }

        vec2 map(in vec3 p)
        {
            // start of generated code
        ";

        static FOOTER: &str = "
        }

        vec3 calculate_normal(in vec3 p)
        {
            const vec3 e = vec3(0.001, 0.0, 0.0);
            vec3 n = vec3(map(p + e.xyy).y - map(p - e.xyy).y,	// Gradient x
                          map(p + e.yxy).y - map(p - e.yxy).y,	// Gradient y
                          map(p + e.yyx).y - map(p - e.yyx).y); // Gradient z

            return normalize(n);
        }

        vec2 raymarch(in ray r)
        {
            float current_total_distance = 0.0;
            float current_id = -1.0;

            for (uint i = 0u; i < MAX_STEPS; ++i)
            {
                vec3 p = r.o + current_total_distance * r.d;
                vec2 hit_info = map(p);
                float id = hit_info.x;
                float dist = hit_info.y;

                current_total_distance += dist;

                if (dist < MIN_HIT_DISTANCE)
                {
                    current_id = id;
                    break;
                }

                if(current_total_distance > MAX_TRACE_DISTANCE)
                {
                    current_total_distance = 0.0;
                    break;
                }
            }
            return vec2(current_id, current_total_distance);
        }

        void main()
        {
            vec2 uv = vs_texcoord * 2.0 - 1.0;
            vec3 camera_position = vec3(0.0, 10.0, 10.0);

            mat3 lookat = lookat(vec3(0.0), camera_position);
            vec3 ro = camera_position;
            vec3 rd = normalize(lookat * vec3(uv.xy, 1.0));
            ray r = ray(ro, rd);

            vec2 res = raymarch(r);
            vec3 hit = ro + rd * res.y;

            const vec3 background = vec3(0.0);
            vec3 color = vec3(0.0);
            switch(int(res.x))
            {
                case 0:
                    vec3 n = calculate_normal(hit);
                    vec3 l = normalize(vec3(1.0, 5.0, 0.0));
                    float d = max(0.0, dot(n, l));
                    color = n * 0.5 + 0.5; //vec3(d);
                    break;
                case 1:
                    // Placeholder
                    break;
                case 2:
                    // Placeholder
                    break;
                    // etc...
                default:
                    color = background;
                    break;
            }

            o_color = vec4(color, 1.0);
        }";

        for uuid in uuids {
            if let Some(op) = graph.get_op(uuid) {
                // Append this op's line of shader code with a leading
                // tab and trailing newline.
                let mut formatted = match op.op_type {

                    OpType::Sphere | OpType::Box | OpType::Plane => {
                        op.op_type.get_formatted(vec![
                            op.name.clone()
                        ])
                    },

                    OpType::Union | OpType::Intersection | OpType::SmoothMinimum => {
                        let uuid_a = op.input_uuids[0];
                        let uuid_b = op.input_uuids[1];
                        op.op_type.get_formatted(vec![
                            op.name.clone(),                                 // This op's name
                            graph.get_op(uuid_a).unwrap().name.clone(), // The name of this op's 1st input
                            graph.get_op(uuid_b).unwrap().name.clone()  // The name of this op's 2nd input
                        ])
                    }

                    OpType::Render => {
                        let uuid = op.input_uuids[0];
                        let name = graph.get_op(uuid).unwrap().name.clone();
                        let mut code = op.op_type.get_formatted(vec![
                            op.name.clone(),    // This op's name
                            name                // The input op's name
                        ]);

                        // Add the final `return` in the `map(..)` function.
                        code.push('\n');
                        code.push('\t');
                        code.push_str(&format!("return vec2(0.0, {});", &op.name[..])[..]);

                        code
                    },

                    _ => "// empty".to_string()
                };
                println!("{}", formatted);

                self.shader_code.push('\t');
                self.shader_code.push_str(&formatted[..]);
                self.shader_code.push('\n');
            }
        }

        let mut fs_src = String::new();
        fs_src.push_str(HEADER);
        fs_src.push_str(&self.shader_code[..]);
        fs_src.push_str(FOOTER);
        println!("Final shader code:");
        println!("{}", fs_src);

        let vs_src = "
        #version 430
        layout(location = 0) in vec2 position;
        layout(location = 1) in vec2 texcoord;
        layout (location = 0) out vec2 vs_texcoord;
        uniform mat4 u_model_matrix;
        uniform mat4 u_projection_matrix;
        void main() {
            vs_texcoord = texcoord;

            gl_Position = u_projection_matrix * u_model_matrix * vec4(position, 0.0, 1.0);
        }".to_string();

        (vs_src, fs_src)
    }
}