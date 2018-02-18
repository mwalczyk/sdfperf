use network::Network;
use operator::{Op, OpType};
use program::Program;

use uuid::Uuid;

pub struct ShaderBuilder {
    shader_code: String,
}

impl ShaderBuilder {
    pub fn new() -> ShaderBuilder {
        ShaderBuilder {
            shader_code: "".to_string(),
        }
    }

    /// Given a list of op indices in the proper post-order, builds
    /// and returns the appropriate shader code.
    pub fn build_sources(&mut self, network: &Network, indices: Vec<usize>) -> Option<Program> {
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

        uniform mat4 u_look_at_matrix;
        uniform vec3 u_camera_position;
        uniform uint u_shading;

        const uint MAX_STEPS = 128u;
        const float MAX_TRACE_DISTANCE = 32.0;
        const float MIN_HIT_DISTANCE = 0.0001;

        struct ray
        {
            vec3 o;
            vec3 d;
        };

        // This will typically be provided by the application,
        // but we leave this function here just in case.
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

        const uint SHADING_CONSTANT = 0;
        const uint SHADING_DIFFUSE = 1;
        const uint SHADING_NORMALS = 2;
        vec3 shading(in vec3 hit)
        {
            if (u_shading == 0)
            {
                return vec3(1.0);
            }
            else
            {
                // calculate normals
                vec3 n = calculate_normal(hit);
                if (u_shading == SHADING_DIFFUSE)
                {
                    const vec3 l = normalize(vec3(1.0, 5.0, 0.0));
                    float d = max(0.0, dot(n, l));
                    return vec3(d);
                }
                else
                {
                    return n * 0.5 + 0.5;
                }
            }
        }

        void main()
        {
            vec2 uv = vs_texcoord * 2.0 - 1.0;

            mat3 lookat = mat3(u_look_at_matrix);
            vec3 ro = u_camera_position;
            vec3 rd = normalize(lookat * vec3(uv.xy, -1.0));
            ray r = ray(ro, rd);

            vec2 res = raymarch(r);
            vec3 hit = ro + rd * res.y;

            const vec3 background = vec3(0.0);
            vec3 color = vec3(0.0);
            switch(int(res.x))
            {
                case 0:
                    color = shading(hit);
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

        // Clear the cached shader code (if there was any).
        self.shader_code = "".to_string();

        // Build the `map` function by traversing the graph of ops.
        for index in indices {
            if let Some(node) = network.graph.get_node(index) {
                // Append this op's line of shader code with a leading
                // tab and trailing newline.
                let mut formatted = match node.data.family {
                    OpType::Sphere | OpType::Box | OpType::Plane => {
                        node.data.family.get_formatted(vec![node.data.name.clone()])
                    }

                    OpType::Union | OpType::Intersection | OpType::SmoothMinimum => {
                        let src_a = network.graph.edges[index].inputs[0];
                        let src_b = network.graph.edges[index].inputs[1];
                        node.data.family.get_formatted(vec![
                            node.data.name.clone(),                                   // This op's name
                            network.graph.get_node(src_a).unwrap().data.name.clone(), // The name of this op's 1st input
                            network.graph.get_node(src_b).unwrap().data.name.clone(), // The name of this op's 2nd input
                        ])
                    }

                    OpType::Render => {
                        let src = network.graph.edges[index].inputs[0];
                        let name = network.graph.get_node(src).unwrap().data.name.clone();
                        let mut code = node.data.family.get_formatted(vec![
                            node.data.name.clone(), // This op's name
                            name,                   // The input op's name
                        ]);

                        // Add the final `return` in the `map(..)` function.
                        code.push('\n');
                        code.push('\t');
                        code.push_str(&format!("return vec2(0.0, {});", &node.data.name[..])[..]);

                        code
                    }

                    _ => "// empty".to_string(),
                };

                // Add a tab indent before each new line of shader code and a newline
                // character after.
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
        println!("{}", self.shader_code);

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
        }"
            .to_string();

        Program::new(vs_src, fs_src)
    }
}
