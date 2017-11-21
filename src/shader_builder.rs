use program::Program;

struct ShaderBuilder {
    shader_code: String
}

impl<'a> ShaderBuilder {
    // Compiles and links a new OpenGL shader program
    // with the same lifetime as this ShaderBuilder
    // instance
    fn build_program(&self) -> Program<'a> {
        static VS_SRC: &'static str = "";
        static FS_SRC: &'static str = "";

        // TODO: insert this ShaderBuilder's shader code into the fragment shader source template

        Program::new(VS_SRC, &self.shader_code[..])
    }
}