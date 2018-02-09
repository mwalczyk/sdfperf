struct ShaderParam<T>(T,);

struct ShaderString {
    formattable_code: String
}

impl ShaderString {

    fn new() -> ShaderString {
        ShaderString {
            formattable_code: "float s = sphere(p, {}, {})".to_string()
        }
    }

    fn format(&self) -> String {
        let indices: Vec<_> = self.formattable_code.match_indices("{}").collect();
        "hello".to_string()
    }
}