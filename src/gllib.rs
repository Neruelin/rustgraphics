#![allow(dead_code)]

use ogl33::*;
use std::fs;
use image::io::Reader as ImageReader;
use ultraviolet::{mat, vec};
use tobj::Model;

// function to wrap clear color and allow it to be labelled safe because nothing should be able to go wrong with glclearcolor
pub fn clear_color(r:f32, g:f32, b:f32, a:f32) {
    unsafe { glClearColor(r,g,b,a) }
}
// verts + norms, tri indices, # of tris, shaderidx
pub struct MeshData(pub Vec<f32>, pub Vec<u32>, pub usize, pub usize);
// vao, vbo, ebo, tris, shaderidx
pub struct Drawable(pub VertexArray, pub Buffer, pub Buffer, pub usize, pub usize);
// impl Drawable {
//     pub fn new() -> Self {

//     }
// }
pub struct DrawableObject(pub vec::Vec3, pub usize, pub usize);

// struct to wrap creation of Vertex Array Objects with functions to bind it as the active VAO or unbind it
pub struct VertexArray(pub GLuint);
impl VertexArray {
    pub fn new() -> Option<Self> {
        let mut vao = 0;
        unsafe { glGenVertexArrays(1, &mut vao) };
        if vao != 0 {
            Some(Self(vao))
        } else {
            None
        }
    }

    pub fn bind(&self) {
        unsafe { glBindVertexArray(self.0) }
    }

    pub fn clear_binding() {
        unsafe { glBindVertexArray(0) }
    }
}

// enum to list buffer types we will use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    // holds arrays of vertices for drawing
    Array = GL_ARRAY_BUFFER as isize,
    // holds indexes of what vertices to use for drawing
    ElementArray = GL_ELEMENT_ARRAY_BUFFER as isize,
}

// struct to wrap creation of buffers with functions to bind the buffer to a target and unbind it
pub struct Buffer(pub GLuint);
impl Buffer {
    pub fn new() -> Option<Self> {
        let mut vbo = 0;
        unsafe { 
            glGenBuffers(1, &mut vbo); 
        }
        if vbo != 0 {
            Some(Self(vbo))
        } else {
            None
        }
    }

    pub fn bind(&self, ty: BufferType) {
        unsafe { glBindBuffer(ty as GLenum, self.0) }
    }

    pub fn unbind(ty: BufferType) {
        unsafe { glBindBuffer(ty as GLenum, 0) }
    }
}

// load data into the bound buffer
pub fn buffer_data(ty: BufferType, data: &[u8], usage: GLenum) {
    unsafe {
        glBufferData(
            ty as GLenum,
            data.len().try_into().unwrap(),
            data.as_ptr().cast(),
            usage,
        );
    }
}

#[derive(PartialEq)]
pub enum ShaderType {
    // shader type for determining and modifying position of geometry on the screen
    Vertex = GL_VERTEX_SHADER as isize,
    // shader type for determining color output of geometry on the screen 
    // possibly other values but usually color
    Fragment = GL_FRAGMENT_SHADER as isize,
}

pub const UNI_ID: [&str; 15] = [
    "translation\0",
    "rotation\0",
    "model\0",
    "view\0",
    "projection\0",
    "lightPos\0",
    "viewPos\0",
    "our_color\0",
    "ambient_color\0",
    "diffuse_color\0",
    "specular_color\0",
    "optical_density\0",
    "dissolve\0",
    "our_texture\0",
    "our_texture2\0"
];
pub enum UniEnum {
    Translation,
    Rotation,
    Model,
    View, 
    Projection,
    LightPos,
    ViewPos,
    Color,
    AmbientColor,
    DiffuseColor,
    SpecularColor,
    OpticalDensity,
    Dissolve,
    Texture,
    Texture2
}

// struct to wrap creation of shader with functions to operate
// the creation, setting of source, compilation, and error detection
// and a function to fully create+compile a shader in a single call
pub struct Shader(pub GLuint);
impl Shader {
    pub fn new(ty: ShaderType) -> Option<Self> {
        let shader = unsafe { glCreateShader(ty as GLenum) };
        if shader != 0 {
            Some(Self(shader))
        } else {
            None
        }
    }
    
    // load shader source code strings into shader object
    // specify number of strings, pointer to array of strings, and array of string lengths
    // all strings will eventually be concatenated and compiled into a single shader program (useful for prepending/appending common shader source fragments) 
    // for this function it only takes one string
    pub fn set_source(&self, src: &str) {
        unsafe {
            glShaderSource(
                self.0,
                1,
                &(src.as_bytes().as_ptr().cast()),
                &(src.len().try_into().unwrap()),
            );
        }
    }

    pub fn compile(&self) {
        unsafe { glCompileShader(self.0) };
    }

    pub fn compile_success(&self) -> bool {
        let mut compiled = 0;
        unsafe { glGetShaderiv(self.0, GL_COMPILE_STATUS, &mut compiled) };
        compiled == i32::from(GL_TRUE)
    }

    pub fn info_log(&self) -> String {
        let mut needed_len = 0;
        unsafe { glGetShaderiv(self.0, GL_INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            glGetShaderInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    pub fn delete(self) {
        unsafe { glDeleteShader(self.0) };
    }

    pub fn from_source(ty: ShaderType, source: &str) -> Result<Self, String> {
        let id = Self::new(ty).ok_or_else(|| "Couldn't allocate new shader".to_string())?;
        id.set_source(source);
        id.compile();
        if id.compile_success() {
            Ok(id)
        } else {
            let out = id.info_log();
            id.delete();
            Err(out)
        }
    }

    pub fn from_file(ty: ShaderType, source: &str) -> Result<Self, String> {
        let shader_source = fs::read_to_string(source)
            .expect("Could not read shader source file.");

        Self::from_source(ty, &shader_source)
    }
}

// struct to wrap creation of shader pipeline from sources
// a complete graphics pipeline combines a vertex and fragment shader
// create a new shader program object
pub struct ShaderProgram(pub GLuint);
impl ShaderProgram {
    pub fn new() -> Option<Self> {
        let prog = unsafe { glCreateProgram() };
        if prog != 0 {
            Some(Self(prog))
        } else {
            None
        }
    }

    pub fn from_sources(vert_source: &str, frag_source: &str) -> Result<Self, String> {
        let mut prog = Self::new().ok_or_else(|| "Couldn't allocate new Shader Program".to_string())?;
        let vert_shader = Shader::from_source(ShaderType::Vertex, vert_source)
            .map_err(|e| format!("Vertex Compile Error: {}", e))?;
        let frag_shader = Shader::from_source(ShaderType::Fragment, frag_source)
            .map_err(|e| format!("Fragment Compile Error: {}", e))?;
        prog.attach_shader(&vert_shader);
        prog.attach_shader(&frag_shader);
        prog.link_program();
        vert_shader.delete();
        frag_shader.delete();
        if prog.link_success() {
            Ok(prog)
        } else {
            let out = format!("Program Link Error: {}", prog.info_log());
            prog.delete();
            Err(out)
        }
    }

    pub fn from_files(vert_source_path: &str, frag_source_path: &str) -> Result<Self, String> {
        let vert_source = fs::read_to_string(vert_source_path)
            .expect("Failed to read vert shader file from source path");
        let frag_source = fs::read_to_string(frag_source_path)
            .expect("Failed to read frag shader file from source path");

        Self::from_sources(&vert_source, &frag_source)
    }

    pub fn from_files_with_texture(vert_source_path: &str, frag_source_path: &str, texture: &Texture, texture_uniform_name: &str) -> Result<Self, String> {
        let prog = Self::from_files(vert_source_path, frag_source_path);
        match prog {
            Ok(prg) => {
                prg.set_int_bool(texture_uniform_name, texture.texture_uniform_id());
                Ok(prg)
            },
            Err(err) => return Err(err),
        }
    }

    pub fn attach_shader(&mut self, shader: &Shader) {
        unsafe { glAttachShader(self.0, shader.0) };
    }

    pub fn link_program(&self) {
        unsafe { glLinkProgram(self.0) };
    }

    pub fn link_success(&self) -> bool {
        let mut success = 0;
        unsafe { glGetProgramiv(self.0, GL_LINK_STATUS, &mut success) };
        success == i32::from(GL_TRUE)
    }

    pub fn info_log(&self) -> String {
        let mut needed_len = 0;
        unsafe { glGetProgramiv(self.0, GL_INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            glGetProgramInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    pub fn use_program(&self) {
        unsafe { glUseProgram(self.0) };
    }

    pub fn delete(&self) {
        unsafe { glDeleteProgram(self.0) };
    }

    pub fn set_int_bool(&self, uniform_name: &str, value: GLint) {
        self.use_program();
        unsafe { 
            glUniform1i(
            glGetUniformLocation(self.0, uniform_name.as_ptr().cast()),
            value
        );}
    }
    
    pub fn set_1_float(&self, uniform_name: &str, value: GLfloat) {
        self.use_program();
        unsafe { 
            glUniform1f(
                glGetUniformLocation(self.0, uniform_name.as_ptr().cast()), 
                value
            ); 
        }
    }

    pub fn set_2_float(&self, uniform_name: &str, v1: GLfloat, v2: GLfloat) {
        self.use_program();
        unsafe { 
            glUniform2f(
                glGetUniformLocation(self.0, uniform_name.as_ptr().cast()), 
                v1, v2
            ); 
        }
    }

    pub fn set_3_float(&self, uniform_name: &str, v1: GLfloat, v2: GLfloat, v3: GLfloat) {
        self.use_program();
        unsafe { 
            glUniform3f(
                glGetUniformLocation(self.0, uniform_name.as_ptr().cast()), 
                v1, v2, v3
            ); 
        }
    }

    pub fn set_4_float(&self, uniform_name: &str, v1: GLfloat, v2: GLfloat, v3: GLfloat, v4: GLfloat) {
        self.use_program();
        unsafe { 
            glUniform4f(
                glGetUniformLocation(self.0, uniform_name.as_ptr().cast()), 
                v1, v2, v3, v4
            ); 
        }
    }

    pub fn set_4_float_matrix(&self, uniform_name: &str, value: *const f32 ) {
        self.use_program();
        unsafe { 
            glUniformMatrix4fv(
                glGetUniformLocation(self.0, uniform_name.as_ptr().cast()), 
                1, GL_FALSE, value
            ); 
        }
    }
}

/* struct to wrap creation of texture with functions to generate texture objects
set textures as active, bind textures, set parameters on textures, load data into textures,
get the texture unit id, create texture from u8 data, create texture from file path */
pub struct Texture(pub GLuint, pub GLenum);
impl Texture {
    pub fn new(texture_unit: GLenum) -> Option<Self> {
        let mut texture = 0;
        unsafe { 
            glGenTextures(1, &mut texture); 
        }
        if texture != 0 {
            Some(Self(texture, texture_unit))
        } else {
            None
        }
    }

    pub fn activate_and_bind(&self) {
        unsafe {
            glActiveTexture(self.1);
        }
        self.bind();
    }

    pub fn bind(&self) {
        unsafe { glBindTexture(GL_TEXTURE_2D, self.0); }
    }

    pub fn bind_and_set_params(&self) {
        self.activate_and_bind();
        unsafe {
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT as GLint);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT as GLint);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as GLint);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as GLint);
        }
    }

    pub fn bind_and_set_data(&self, h: i32, w: i32, data: &[u8], rgba: bool) {
        self.activate_and_bind();
        unsafe {
            if rgba {
                glTexImage2D(
                    GL_TEXTURE_2D, 0, GL_RGBA as i32, 
                    w, h, 0, GL_RGBA, 
                    GL_UNSIGNED_BYTE, data.as_ptr().cast()
                );
            } else {
                glTexImage2D(
                    GL_TEXTURE_2D, 0, GL_RGB as i32, 
                    w, h, 0, GL_RGB, 
                    GL_UNSIGNED_BYTE, data.as_ptr().cast()
                );
            }
            glGenerateMipmap(GL_TEXTURE_2D);
        }
    }

    pub fn unbind(&self) {
        unsafe { glBindTexture(GL_TEXTURE_2D, 0); }
    }

    pub fn delete(&self) {
        unsafe { glDeleteTextures(1, &(self.0)); }
    }

    pub fn texture_uniform_id(&self) -> i32 {
        (self.1 - GL_TEXTURE0) as i32
    }

    pub fn from_data(texture_unit: GLenum, h: i32, w: i32, data: &[u8], rgba: bool) -> Self {
        let texture = Self::new(texture_unit).expect("Couldn't create new Texture");
        texture.bind_and_set_params();
        texture.bind_and_set_data(h, w, data, rgba);
        texture
    }

    pub fn from_file(texture_unit: GLenum, texture_file_path: &str, rgba: bool) -> Self {
        let dynimg = ImageReader::open(texture_file_path).unwrap().decode().unwrap().flipv();
        let imgheight = dynimg.height() as i32;
        let imgwidth = dynimg.width() as i32;
        let imgdat = dynimg.as_bytes();
        Self::from_data(texture_unit, imgheight, imgwidth, imgdat, rgba)
    }
}

pub fn color_program<'a>(
    base_folder: &'a str,
    shader_folder: &'a str,
    color: &vec::Vec3,
    translation: &mat::Mat4, 
    rotation: &mat::Mat4, 
    model: &mat::Mat4, 
    view: &mat::Mat4, 
    projection: &mat::Mat4
) -> ShaderProgram {
    let vert = format!("{}/{}/{}", base_folder, shader_folder, "vertex");
    let frag = format!("{}/{}/{}", base_folder, shader_folder, "fragment");
    let shader = ShaderProgram::from_files(&vert, &frag).unwrap();
    let [v1, v2, v3] = *((*color).as_array());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Translation as usize], translation.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], rotation.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Model as usize], model.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::View as usize], view.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Projection as usize], projection.as_ptr().cast());
    shader.set_3_float(UNI_ID[UniEnum::Color as usize], v1, v2, v3);
    shader.set_3_float(UNI_ID[UniEnum::LightPos as usize], 0.0, 0.0, 0.0);
    shader.set_3_float(UNI_ID[UniEnum::ViewPos as usize], 0.0, 0.0, 0.0);
    shader
}

pub fn param_color_program<'a>(
    base_folder: &'a str,
    shader_folder: &'a str,
    optical_density: f32,
    ambient_color: &vec::Vec3,
    diffuse_color: &vec::Vec3,
    specular_color: &vec::Vec3,
    dissolve: f32,
    translation: &mat::Mat4, 
    rotation: &mat::Mat4, 
    model: &mat::Mat4, 
    view: &mat::Mat4, 
    projection: &mat::Mat4
) -> ShaderProgram {
    let vert = format!("{}/{}/{}", base_folder, shader_folder, "vertex");
    let frag = format!("{}/{}/{}", base_folder, shader_folder, "fragment");
    let shader = ShaderProgram::from_files(&vert, &frag).unwrap();
    let [v1, v2, v3] = *((*ambient_color).as_array());
    let [v4, v5, v6] = *((*diffuse_color).as_array());
    let [v7, v8, v9] = *((*specular_color).as_array());
    println!("ambient {:?}", (v1, v2, v3));
    println!("diffuse {:?}", (v4, v5, v6));
    println!("specular {:?}", (v7, v8, v9));
    shader.set_3_float(UNI_ID[UniEnum::AmbientColor as usize], v1, v2, v3);
    shader.set_3_float(UNI_ID[UniEnum::DiffuseColor as usize], v4, v5, v6);
    shader.set_3_float(UNI_ID[UniEnum::SpecularColor as usize], v7, v8, v9);
    shader.set_1_float(UNI_ID[UniEnum::OpticalDensity as usize], optical_density);
    shader.set_1_float(UNI_ID[UniEnum::Dissolve as usize], dissolve);
    shader.set_4_float_matrix(UNI_ID[UniEnum::Translation as usize], translation.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], rotation.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Model as usize], model.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::View as usize], view.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Projection as usize], projection.as_ptr().cast());
    shader.set_3_float(UNI_ID[UniEnum::LightPos as usize], 0.0, 0.0, 0.0);
    shader.set_3_float(UNI_ID[UniEnum::ViewPos as usize], 0.0, 0.0, 0.0);
    shader
}

pub fn texture_program<'a>(
    base_folder: &'a str, 
    shader_folder: &'a str,
    texture: &'a Texture,
    translation: &mat::Mat4, 
    rotation: &mat::Mat4, 
    model: &mat::Mat4, 
    view: &mat::Mat4, 
    projection: &mat::Mat4
) -> ShaderProgram {
    let vert = format!("{}/{}/{}", base_folder, shader_folder, "vertex");
    let frag = format!("{}/{}/{}", base_folder, shader_folder, "fragment");
    let shader = ShaderProgram::from_files_with_texture(
        &vert, 
        &frag,
        &texture,
        UNI_ID[UniEnum::Texture as usize]
    ).unwrap();
    shader.set_4_float_matrix(UNI_ID[UniEnum::Translation as usize], translation.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], rotation.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Model as usize], model.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::View as usize], view.as_ptr().cast());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Projection as usize], projection.as_ptr().cast());
    shader
}

pub type Vertex = [f32; 3];
pub type TexelVertex = [f32; 3 + 2];
pub type NormalVertex = [f32; 3 + 3];

pub struct Mesh(pub Vec<TexelVertex>);
impl Mesh {
    pub fn new() -> Mesh {
        Self(Vec::new())
    }

    pub fn add_tri(&mut self, tris: &mut Vec<TexelVertex>) {
        self.0.append(tris);
    }
}

pub fn combine_loaded_data<'a> (
    loaded_data: &'a Model,
) -> Vec<f32> {
    let mut output_vec = Vec::new();
    let num = (*loaded_data).mesh.positions.len();
    for i in 0..num {
        output_vec.push((*loaded_data).mesh.positions[i]);
        if i % 3 == 2 {
            for j in (i-2)..=(i) {
                output_vec.push((*loaded_data).mesh.normals[j]);
            }
        }
    }

    output_vec
}

/// The polygon display modes you can set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
  /// Just show the points.
  Point = GL_POINT as isize,
  /// Just show the lines.
  Line = GL_LINE as isize,
  /// Fill in the polygons.
  Fill = GL_FILL as isize,
}

/// Sets the font and back polygon mode to the mode given.
pub fn polygon_mode(mode: PolygonMode) {
  unsafe { glPolygonMode(GL_FRONT_AND_BACK, mode as GLenum) };
}