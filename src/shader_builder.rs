use network::Network;
use operator::{Op, OpType};
use program::Program;
use shader_string::ShaderString;

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
        static HEADER: &str = "
        #version 430
        layout (location = 0) in vec2 vs_texcoord;
        layout (location = 0) out vec4 o_color;

        uniform mat4 u_look_at_matrix;
        uniform vec3 u_camera_position;
        uniform vec3 u_camera_front;
        uniform uint u_shading;
        uniform float u_time;

        // The SSBO that will contain a transform for each op in the
        // graph. Note that according to the spec, there can only be
        // one array of variable size per SSBO, which is why we use
        // the convenience struct `transform` above.
        //
        // Here, we pack each transform into a single `vec4` where
        // the xyz components represent a translation and the w
        // component represents a uniform scale.
        layout (std430, binding = 0) buffer transforms_block
        {
            vec4 transforms[];
        };

        const uint MAX_STEPS = 128u;
        const float MAX_TRACE_DISTANCE = 32.0;
        const float MIN_HIT_DISTANCE = 0.01;

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

        float op_subtract(float a, float b)
        {
            return max(-a, b);
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
            // start of generated cod-
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

        float ambient_occlusion(in vec3 p, in vec3 n)
        {
            const float attenuation = 0.5;
            float ao;
            float accum = 0.0;
            float scale = 1.0;
            for(int step = 0; step < 5; step++)
            {
                float hr = 0.01 + 0.02 * float(step * step);
                vec3 aopos = n * hr + p;

                float dist = map(aopos).y;
                ao = -(dist - hr);
                accum += ao * scale;
                scale *= attenuation;
            }
            ao = 1.0 - clamp(accum, 0.0, 1.0);

            return ao;
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
                    float ao = ambient_occlusion(hit, n);
                    return vec3(pow(ao, 3.0));
                }
                else
                {
                    return n * 0.5 + 0.5;
                }
            }
        }

        ray generate_ray()
        {
            // uv-coordinates in the range [-1..1]
            vec2 uv = vs_texcoord * 2.0 - 1.0;

            const float PI = 3.14159265359;
            const float fov = 50.0;
            const float fovx = PI * fov / 360.0;
            float fovy = fovx * 1.0; // iResolution.y/iResolution.x;
            float ulen = tan(fovx);
            float vlen = tan(fovy);

            const vec3 camera_up = vec3(0.0, 1.0, 0.0);
            vec2 cam_uv = uv;
            vec3 camera_right = normalize(cross(camera_up, u_camera_front));
            vec3 pixel = u_camera_position + u_camera_front + camera_right * cam_uv.x * ulen + camera_up * cam_uv.y * vlen;

            vec3 ro = u_camera_position;
            vec3 rd = normalize(pixel - u_camera_position);

            return ray(ro, rd);
        }

        void main()
        {
            ray r = generate_ray();

            vec2 res = raymarch(r);
            vec3 hit = r.o + r.d * res.y;

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

                let mut formatted = match node.data.family {

                    OpType::Sphere | OpType::Box | OpType::Plane => {
                        let shader_code = ShaderString::new(
                            node.data.family.get_code_template(),
                            &node.data.name,
                            Some(node.data.transform_index),
                            None,
                            None,
                        );
                        shader_code.to_string()
                    }

                    OpType::Union
                    | OpType::Subtraction
                    | OpType::Intersection
                    | OpType::SmoothMinimum => {
                        // If this operator doesn't have at least 2 inputs,
                        // then we exit early, since this isn't a valid
                        // shader graph.
                        if network.graph.edges[index].inputs.len() < 2 {
                            return None;
                        }
                        let a = network.graph.edges[index].inputs[0];
                        let b = network.graph.edges[index].inputs[1];
                        let shader_code = ShaderString::new(
                            node.data.family.get_code_template(),
                            &node.data.name,
                            Some(node.data.transform_index),
                            Some(&network.graph.get_node(a).unwrap().data.name),
                            Some(&network.graph.get_node(b).unwrap().data.name),
                        );
                        shader_code.to_string()
                    }

                    OpType::Render => {
                        if network.graph.edges[index].inputs.len() < 1 {
                            return None;
                        }
                        let a = network.graph.edges[index].inputs[0];
                        let mut shader_code = ShaderString::new(
                            node.data.family.get_code_template(),
                            &node.data.name,
                            Some(node.data.transform_index),
                            Some(&network.graph.get_node(a).unwrap().data.name),
                            None,
                        );

                        // Add the final `return` in the `map(..)` function.
                        shader_code.code.push('\n');
                        shader_code.code.push('\t');
                        shader_code
                            .code
                            .push_str(&format!("return vec2(0.0, {});", &node.data.name));
                        shader_code.to_string()
                    }
                };

                // Add a tab indent before each new line of shader code and a newline
                // character after.
                self.shader_code.push('\t');
                self.shader_code.push_str(&formatted);
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
