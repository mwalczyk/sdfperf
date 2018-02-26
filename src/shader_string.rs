pub struct ShaderString {
    pub code: String,
}

impl ShaderString {
    pub fn new(
        mut code: String,
        name: &str,
        index: Option<usize>,
        input_a: Option<&str>,
        input_b: Option<&str>,
    ) -> ShaderString {
        code = code.replace("NAME", name);

        if let Some(i) = index {
            code = code.replace("INDEX", &i.to_string());
        }
        if let Some(a) = input_a {
            code = code.replace("INPUT_A", a);
        }
        if let Some(b) = input_b {
            code = code.replace("INPUT_B", b);
        }

        ShaderString { code }
    }

    pub fn to_string(self) -> String {
        self.code
    }
}

#[test]
fn test_formatting() {
    let mut unformatted = "
        float s_NAME = transforms[INDEX].w;
        vec3 t_NAME = transforms[INDEX].xyz;
        float NAME = sdf_sphere(p / s_NAME + t_NAME, vec3(0.0), 1.0) * s_NAME;
        "
        .to_string();

    let actual = "
        float s_sphere = transforms[0].w;
        vec3 t_sphere = transforms[0].xyz;
        float sphere = sdf_sphere(p / s_sphere + t_sphere, vec3(0.0), 1.0) * s_sphere;
        "
        .to_string();

    let formatted = ShaderString::new(unformatted, "sphere", Some("0"), None, None).code;
    assert_eq!(formatted, actual);
}
