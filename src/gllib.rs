#![allow(dead_code)]

use log::{Level, SetLoggerError, LevelFilter, info};
use ogl33::*;
use beryllium::*;
// use rapier2d::prelude::*;
// use rapier2d::math::Vector;
use rapier2d::{prelude::*, pipeline::ChannelEventCollector, crossbeam};
use std::fs;
use std::cell::RefCell;
use std::collections::{HashSet, HashMap};
use std::str::FromStr;
// use rand::Rng;
use image::io::Reader as ImageReader;
use ultraviolet::{mat, vec, projection};
use tobj::Model;
use core::{
    convert::TryInto,
    mem::size_of
};
use std::time::{Instant, Duration};
use crate::behaviors::*;
use crate::camera::*;

// function to wrap clear color and allow it to be labelled safe because nothing should be able to go wrong with glclearcolor
pub fn clear_color(r:f32, g:f32, b:f32, a:f32) {
    unsafe { glClearColor(r,g,b,a) }
}

pub const LT_MAIN_LOOP: &str = "MainLoop";
pub const LT_BEHAVIORS: &str = "Behaviors";

pub struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            match record.target() {
                LT_MAIN_LOOP => {},
                LT_BEHAVIORS => {},
                _ => {return;}
            }
            println!("[{}:{}]: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

pub fn init_log() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
}

pub struct CameraParams {
    pub view_pos: vec::Vec3,
    pub view_rot: vec::Vec3,
    pub light_position: vec::Vec3,
    pub projection: mat::Mat4
}
impl CameraParams {
    pub fn new(view_pos: vec::Vec3, view_rot: vec::Vec3, light_position: vec::Vec3, projection: mat::Mat4) -> Self {
        Self {view_pos, view_rot, light_position, projection}
    }
    pub fn look_dir(&self) -> vec::Vec3 {
        let [_roll, pitch, yaw] = *self.view_rot.as_array();
        let x = f32::cos(yaw.to_radians()) * f32::cos(pitch.to_radians());
        let y = f32::sin(pitch.to_radians());
        let z = f32::sin(yaw.to_radians()) * f32::cos(pitch.to_radians());
        let dir = vec::Vec3::new(x, y, z);
        dir.normalized()
    }
    pub fn view_matrix(&self) -> mat::Mat4 {
        let look_dir = self.look_dir();
        let rot = mat::Mat4::look_at(self.view_pos, self.view_pos + look_dir, vec::Vec3::new(0.0,1.0,0.0));
        
        rot
    }
}

// verts + norms, tri indices, # of tris, shaderidx
pub struct MeshData {
    pub point_data: Vec<f32>, 
    pub point_indices: Vec<u32>, 
    pub tri_count: usize, 
    pub shader_idx: usize
}
pub struct MeshDataGroup(pub Vec<MeshData>);
// vao, vbo, ebo, tris, shaderidx
pub struct Drawable{
    pub vao: VertexArray, 
    pub vbo: Buffer, 
    pub ebo: Buffer, 
    pub tri_count: usize, 
    pub shader_idx: usize
}
pub struct DrawableGroup(pub Vec<Drawable>);
pub struct DrawableObject {
    pub position: vec::Vec3, 
    pub rotation: vec::Vec3,
    pub scale: vec::Vec3,
    pub drawable_group_idx: usize, 
}
impl DrawableObject {
    pub fn new(position: vec::Vec3, rotation: vec::Vec3, scale: vec::Vec3, drawable_group_idx: usize) -> Self {
        Self {position, rotation, scale, drawable_group_idx}
    }
    pub fn model_matrix(&self) -> mat::Mat4 {
        let pos = mat::Mat4::from_translation(self.position);
        let [roll, pitch, yaw] = *self.rotation.as_array();
        let rot = mat::Mat4::from_euler_angles(roll, pitch, yaw);
        let sca = mat::Mat4::from_nonuniform_scale(self.scale);
        sca * pos * rot
    }
    pub fn rotation_matrix(&self) -> mat::Mat4 {
        let [roll, pitch, yaw] = *self.rotation.as_array();
        mat::Mat4::from_euler_angles(roll, pitch, yaw)
    }
}

pub struct GameObject<T> {
    pub position: vec::Vec3,
    pub rotation: vec::Vec3,
    pub scale: vec::Vec3,
    pub children: Vec<Self>,
    pub drawable_object: Option<DrawableObject>,
    pub rigid_body_handle: Option<RigidBodyHandle>,
    pub behaviors: HashSet<Behaviors>,
    pub collision_behaviors: HashSet<CollisionBehaviors>,
    pub behaviors_data: HashMap<BehaviorData, T>,
    pub collisions_behavior_data: HashMap<CollisionData, Box<dyn CollisionDataContainer>>,
    pub id: GameObjectID,
    pub grounded: bool,
}
impl<T> GameObject<T> {
    pub fn new(
        position: vec::Vec3, 
        rotation: vec::Vec3, 
        scale: vec::Vec3, 
        children: Vec<Self>, 
        drawable_object: Option<DrawableObject>, 
        rigid_body_handle: Option<RigidBodyHandle>, 
        behaviors: HashSet<Behaviors>, 
        collision_behaviors: HashSet<CollisionBehaviors>,
        behaviors_data: HashMap<BehaviorData, T>,
        collisions_behavior_data: HashMap<CollisionData, Box<dyn CollisionDataContainer>>,
    ) -> Self {
        Self{
            position, 
            rotation, 
            scale, 
            children, 
            drawable_object, 
            rigid_body_handle, 
            behaviors, 
            collision_behaviors, 
            behaviors_data,
            collisions_behavior_data,
            id: 0, 
            grounded: true
        }
    }
    pub fn empty() -> Self {
        Self{
            position: vec::Vec3::zero(), 
            rotation: vec::Vec3::zero(), 
            scale: vec::Vec3::one(), 
            children: vec![], 
            drawable_object: None, 
            rigid_body_handle: None, 
            behaviors: HashSet::new(), 
            collision_behaviors: HashSet::new(), 
            behaviors_data: HashMap::new(),
            collisions_behavior_data: HashMap::new(),
            id: 0, 
            grounded: true
        }
    }
    pub fn model_matrix(&self) -> mat::Mat4 {
        let pos = mat::Mat4::from_translation(self.position);
        let [roll, pitch, yaw] = *self.rotation.as_array();
        let rot = mat::Mat4::from_euler_angles(roll, pitch, yaw);
        let sca = mat::Mat4::from_nonuniform_scale(self.scale);
        sca * pos * rot
    }
    pub fn rotation_matrix(&self) -> mat::Mat4 {
        let [roll, pitch, yaw] = *self.rotation.as_array();
        mat::Mat4::from_euler_angles(roll, pitch, yaw)
    }
    pub fn physic_update(&mut self, rigid_body_set: &RigidBodySet) {
        if let Some(rigid_body_idx) = &mut self.rigid_body_handle {
            self.position.x = rigid_body_set[*rigid_body_idx].translation().x;
            self.position.y = rigid_body_set[*rigid_body_idx].translation().y;
            self.rotation.z = rigid_body_set[*rigid_body_idx].rotation().angle();
        }
    }
    pub fn add_behavior(mut self, behavior: Behaviors) -> Self {
        self.behaviors.insert(behavior);
        self
    }
    pub fn add_collision_behavior(mut self, collision_behavior: CollisionBehaviors) -> Self {
        self.collision_behaviors.insert(collision_behavior);
        self
    }
    pub fn add_behavior_data(mut self, behavior: Behaviors, behavior_data: T) -> Self {
        self.behaviors_data.insert(BehaviorData::Behaviors(behavior), behavior_data);
        self
    }
    pub fn add_collision_behavior_data(mut self, collision_behavior: CollisionBehaviors, collision_behavior_data: Box<dyn CollisionDataContainer>) -> Self {
        self.collisions_behavior_data.insert( CollisionData::CollisionBehaviors(collision_behavior), collision_behavior_data);
        self
    }
    pub fn add_child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }
}

pub type GameObjectID = usize;
pub struct GameObjectStore<T>(pub HashMap<GameObjectID, RefCell<GameObject<T>>>, pub HashMap<RigidBodyHandle, GameObjectID>, GameObjectID);
impl<T> GameObjectStore<T> {
    pub fn new() -> Self {
        Self(HashMap::new(), HashMap::new(), 0)
    }
    pub fn add(&mut self, mut go: GameObject<T>) -> GameObjectID {
        self.2 += 1;
        go.id = self.2;
        if let Some(rb_handle) = go.rigid_body_handle {
            self.1.insert(rb_handle, self.2);
        }
        self.0.insert(self.2, RefCell::new(go));
        self.2
    }
    pub fn remove(&mut self, id: &GameObjectID) {
        if self.0.contains_key(id) {
            if let Some(rb_handle) = self.0[&id].borrow().rigid_body_handle {
                self.1.remove(&rb_handle);
            }
            self.0.remove(id);
        } else {
            // some error i guess
        }
    }
    pub fn lookup_by_rb_handle(&self, rb_handle: &RigidBodyHandle) -> GameObjectID {
        *self.1.get(rb_handle).expect("rb_handle not in rb to obj map")
    }
}

pub fn make_go<T>(position: vec::Vec3, rotation: vec::Vec3, scale: vec::Vec3, model_position: vec::Vec3, model_rotation: vec::Vec3, model_scale: vec::Vec3, drawable_obj_idx: usize) -> GameObject<T> {
    let mut go = GameObject::empty();
    go.position = position;
    go.rotation = rotation;
    go.scale = scale;
    go.drawable_object = Some(DrawableObject::new(model_position, model_rotation, model_scale, drawable_obj_idx));
    go
}

pub fn make_go_rb<T>(position: vec::Vec3, rotation: vec::Vec3, scale: vec::Vec3, model_position: vec::Vec3, model_rotation: vec::Vec3, model_scale: vec::Vec3, drawable_obj_idx: usize, rigid_body_handle: RigidBodyHandle) -> GameObject<T> {
    let mut go = GameObject::empty();
    go.position = position;
    go.rotation = rotation;
    go.scale = scale;
    go.drawable_object = Some(DrawableObject::new(model_position, model_rotation, model_scale, drawable_obj_idx));
    go.rigid_body_handle = Some(rigid_body_handle);
    go
}

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

pub const UNI_ID: [&str; 14] = [
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
    model: &mat::Mat4, 
    view: &mat::Mat4, 
    projection: &mat::Mat4
) -> ShaderProgram {
    let vert = format!("{}/{}/{}", base_folder, shader_folder, "vertex.GLSL");
    let frag = format!("{}/{}/{}", base_folder, shader_folder, "fragment.GLSL");
    let shader = ShaderProgram::from_files(&vert, &frag).unwrap();
    let [v1, v2, v3] = *((*color).as_array());
    shader.set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], mat::Mat4::identity().as_ptr().cast());
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
    model: &mat::Mat4, 
    view: &mat::Mat4, 
    projection: &mat::Mat4
) -> ShaderProgram {
    let vert = format!("{}/{}/{}", base_folder, shader_folder, "vertex.GLSL");
    let frag = format!("{}/{}/{}", base_folder, shader_folder, "fragment.GLSL");
    let shader = ShaderProgram::from_files(&vert, &frag).unwrap();
    let [v1, v2, v3] = *((*ambient_color).as_array());
    let [v4, v5, v6] = *((*diffuse_color).as_array());
    let [v7, v8, v9] = *((*specular_color).as_array());
    shader.set_3_float(UNI_ID[UniEnum::AmbientColor as usize], v1, v2, v3);
    shader.set_3_float(UNI_ID[UniEnum::DiffuseColor as usize], v4, v5, v6);
    shader.set_3_float(UNI_ID[UniEnum::SpecularColor as usize], v7, v8, v9);
    shader.set_1_float(UNI_ID[UniEnum::OpticalDensity as usize], optical_density);
    shader.set_1_float(UNI_ID[UniEnum::Dissolve as usize], dissolve);
    shader.set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], mat::Mat4::identity().as_ptr().cast());
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
    model: &mat::Mat4, 
    view: &mat::Mat4, 
    projection: &mat::Mat4
) -> ShaderProgram {
    let vert = format!("{}/{}/{}", base_folder, shader_folder, "vertex.GLSL");
    let frag = format!("{}/{}/{}", base_folder, shader_folder, "fragment.GLSL");
    let shader = ShaderProgram::from_files_with_texture(
        &vert, 
        &frag,
        &texture,
        UNI_ID[UniEnum::Texture as usize]
    ).unwrap();
    shader.set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], mat::Mat4::identity().as_ptr().cast());
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
    let num = loaded_data.mesh.positions.len();
    for i in 0..num {
        output_vec.push(loaded_data.mesh.positions[i]);
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

pub fn get_rb_handle_from_collision_handle(
    collider_set: &ColliderSet, 
    collider_handle: &ColliderHandle
) -> Option<RigidBodyHandle> {
    collider_set.get(*collider_handle).unwrap().parent()
}

pub fn init_sdl() -> SDL {
    let sdl = SDL::init(InitFlags::Everything).expect("couldn't start SDL");

    sdl.gl_set_attribute(SdlGlAttr::MajorVersion, 3).unwrap();
    sdl.gl_set_attribute(SdlGlAttr::MinorVersion, 3).unwrap();
    sdl.gl_set_attribute(SdlGlAttr::Profile, GlProfile::Core).unwrap();
    #[cfg(target_os = "macos")]
    {
        sdl
        .gl_set_attribute(SdlGlAttr::Flags, ContextFlag::ForwardCompatible)
        .unwrap();
    }

    return sdl;
}

pub struct LoopContext<'a, T> {
    pub go: &'a mut GameObject<T>, 
    pub keys_held: &'a HashSet<Keycode>,
    pub mouse_deltas: &'a (f32, f32), 
    pub camera: &'a mut CameraParams,
    pub deltasecs: f32, 
    pub game_time: f32, 
    pub rigid_body_set: &'a mut RigidBodySet, 
    pub collider_set: &'a mut ColliderSet,
    pub floor_set: &'a mut HashSet<RigidBodyHandle>,
    pub model_map: &'a HashMap<&'a str, usize>,
    pub game_obj_store: &'a GameObjectStore<T>
}

pub struct Context<T> {
    pub sdl: SDL,
    pub window: GlWindow,
    pub camera: CameraParams,
    pub shader_folder_path: String,
    pub param_blinn_phong_shader_folder: String,
    pub shaders: Vec<ShaderProgram>,
    pub shader_map: HashMap<String, usize>,
    pub drawable_groups: Vec<DrawableGroup>,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub floor_set: HashSet<RigidBodyHandle>,
    pub game_obj_store: GameObjectStore<T>,
    pub pre_draw: Box<dyn Fn(&ShaderProgram, &DrawableObject)>,
    pub handleit: Box<dyn Fn(&mut LoopContext<T>) -> (Vec<GameObjectID>, Vec<GameObject<T>>)>,
    pub handlecollisionit: Box<dyn Fn(&mut LoopContext<T>, &RefCell<GameObject<T>>) -> (Vec<GameObjectID>, Vec<GameObject<T>>)>,
}
impl<T> Context<T> {
    pub fn new(window_width: u32, window_height: u32) -> Result<Self, String> {
        let def_shader_folder_path = String::from_str("src/shaders").expect("string failed");
        let def_param_blinn_phong_shader_folder = String::from_str("param_blinn_phong_shader").expect("string failed");

        let sdl = init_sdl();
        let camera = CameraParams::new(
            vec::Vec3::zero(),
            vec::Vec3::new(0.0, 0.0, 90.0),
            vec::Vec3::new(0.0, 10.0, -10.0),
            projection::perspective_gl(45.0_f32, (window_width as f32) / (window_height as f32), 0.1, 100.0)
        );
        
        let window = sdl.create_gl_window("OpenGL", WindowPosition::Centered, window_width, window_height, WindowFlags::Shown);
        match window {
            Ok(window) => {
                let ctx = Context{
                    sdl,
                    window,
                    camera,
                    shader_folder_path: def_shader_folder_path,
                    param_blinn_phong_shader_folder: def_param_blinn_phong_shader_folder,
                    shaders: vec![],
                    shader_map: HashMap::new(),
                    // meshes: vec![],
                    drawable_groups: vec![],
                    rigid_body_set: RigidBodySet::new(),
                    collider_set: ColliderSet::new(),
                    floor_set: HashSet::new(),
                    game_obj_store: GameObjectStore::new(),
                    pre_draw: Box::new(move |_shader: &ShaderProgram, _draw: &DrawableObject| {}),
                    handleit: Box::new(move |_loop_context: &mut LoopContext<T>| -> (Vec<GameObjectID>, Vec<GameObject<T>>) {(vec![], vec![])}),
                    handlecollisionit: Box::new(move |_loop_context: &mut LoopContext<T>, _other: &RefCell<GameObject<T>>| -> (Vec<GameObjectID>, Vec<GameObject<T>>) {(vec![], vec![])})
                };
                
                /* set vsync on to block program until rendered screen has been shown */
                // ctx.window.set_swap_interval(SwapInterval::Vsync);
                ctx.init_ogl();
                
                Ok(ctx)
            },
            Err(e) => {
                Err(e)
            }
        }
    }
    pub fn init_ogl(&self) {
        unsafe {
            load_gl_with(|f_name| self.window.get_proc_address(f_name));
    
            glEnable(GL_DEPTH_TEST);
        }
        
        clear_color(0.0, 0.0, 0.0, 1.0);
    }
    pub fn load_model(&mut self, model_path: &str) -> usize {
        let (models, _materials) = tobj::load_obj(model_path, &tobj::GPU_LOAD_OPTIONS).expect("Failed to load model");
        let mats = _materials.expect("Failed to read mtl when loading model materials");

        for mat in &mats {
            let mat_name = (*mat).name.clone();
            if !self.shader_map.contains_key(&mat_name) {
                
                let new_shader_idx = self.shaders.len();
                
                self.shaders.push(param_color_program(
                    self.shader_folder_path.as_str(), 
                    self.param_blinn_phong_shader_folder.as_str(), 
                    (*mat).optical_density,
                    &vec::Vec3::from((*mat).ambient),
                    &vec::Vec3::from((*mat).diffuse),
                    &vec::Vec3::from((*mat).specular),
                    (*mat).dissolve,
                    &mat::Mat4::identity(),
                    &self.camera.view_matrix(),
                    &self.camera.projection
                ));

                self.shader_map.insert(mat_name, new_shader_idx);
            }
        }

        let mut mesh_data_group: Vec<MeshData> = vec![];

        for model in models {
            let tris = model.mesh.indices.len();
            let mat_id = model.mesh.material_id.unwrap();
            mesh_data_group.push(MeshData{
                point_data: combine_loaded_data(&model), 
                point_indices: model.mesh.indices.clone(), 
                tri_count: tris,
                shader_idx: self.shader_map[mats[mat_id].name.as_str()]
            });
        }
        // let mesh_id = self.meshes.len();
        let mesh_data = MeshDataGroup(mesh_data_group);

        let mut drawable_group = vec![];
        for mesh in mesh_data.0 {
            /* generate a Vertex Array Object and store ref in mutable vao variable
            and bind the VAO making it the active */
            let (vao, vbo, ebo) = {
                let vao = VertexArray::new().expect("Couldn't make a new VAO");
                vao.bind();
            
                /* generate a Buffer Object and store req in mutable vbo variable
                and set the given buffer as the current active Array Buffer 
                and then initializes target's active buffer's storage with a size and initial data and a hint to its usage
                and then initialize active Array Buffer with size of vertex array and pointer to vertex array */
                let vbo = {
                    let vbo = Buffer::new().expect("Couldn't make a new buffer");
                    vbo.bind(BufferType::Array);
                    // buffer_data(BufferType::Array, bytemuck::cast_slice(&vertices), GL_STATIC_DRAW);
                    buffer_data(BufferType::Array, bytemuck::cast_slice(mesh.point_data.as_slice()), GL_STATIC_DRAW);
                    vbo
                    };

                /* generate buffer to hold groups of vertexes that form triangles
                set as the active element array buffer type
                load in the data */
                let ebo = {
                    let ebo = Buffer::new().expect("Couldn't make a new buffer");
                    ebo.bind(BufferType::ElementArray);
                    buffer_data(BufferType::ElementArray, bytemuck::cast_slice(mesh.point_indices.as_slice()), GL_STATIC_DRAW);
                    ebo
                    };

                unsafe {
                    glVertexAttribPointer(
                        0,
                        3,
                        GL_FLOAT,
                        GL_TRUE,
                        size_of::<NormalVertex>().try_into().unwrap(),
                        size_of::<[f32; 0]>() as *const _,
                    );
                    glEnableVertexAttribArray(0);
                    glVertexAttribPointer(
                        1,
                        3,
                        GL_FLOAT,
                        GL_FALSE,
                        size_of::<NormalVertex>().try_into().unwrap(),
                        (size_of::<f32>() * 3) as *const _,
                    );
                    glEnableVertexAttribArray(1);
                }

                (vao, vbo, ebo)
            };
            drawable_group.push(Drawable{
                vao, 
                vbo, 
                ebo, 
                tri_count: mesh.tri_count, 
                shader_idx: mesh.shader_idx
            });
        }
        let draw_id = self.drawable_groups.len();
        self.drawable_groups.push(DrawableGroup(drawable_group));

        draw_id
    }
}

// pub fn 

pub fn main_loop<T> (ctx: &mut Context<T>, model_map: &HashMap<&str, usize>) {
    init_log().expect("");
    info!(target: LT_MAIN_LOOP, "main_loop function called");
    let mut _rng = rand::thread_rng();
    
    /* Physics Config */
    let gravity = vector![0.0, -9.81];
    let mut integration_parameters = IntegrationParameters::default();
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = BroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let (collision_send, collision_recv) = crossbeam::channel::unbounded();
    let event_handler = ChannelEventCollector::new(collision_send);
    let physics_hooks = ();

    /* mouse input config */
    const MOUSE_SENSITIVITY: f32 = 0.4;
    ctx.sdl.set_relative_mouse_mode(true).unwrap();

    /* Keyboard input storage */
    let mut keys_held = HashSet::new();

    /* Time and FPS configuration */
    let mut deltatime = Duration::new(0, 0);
    let target_fps: f32 = 60.0;
    let target_frame_micros = (1000000_f32 / target_fps).ceil() as u64;
    let _target_frame_time = Duration::from_micros(target_frame_micros);
    let start_instant = Instant::now();
    let mut update_view_lights = true;

    'main_loop: loop {
        let frame_start = Instant::now();
        let deltasecs = deltatime.as_secs_f32();
        let game_time = start_instant.elapsed().as_secs_f32();
        let mut mouse_deltas = (0.0, 0.0);

        while let Some(event) = ctx.sdl.poll_events().and_then(Result::ok) {
            match event {
                Event::Quit(_) => break 'main_loop,
                Event::Keyboard(KeyboardEvent {
                    is_pressed,
                    key: KeyInfo {keycode, ..},
                    ..
                }) => {
                    if is_pressed {
                        keys_held.insert(keycode);
                    } else {
                        keys_held.remove(&keycode);
                    }
                },
                Event::MouseMotion(MouseMotionEvent { x_delta, y_delta, .. }) => {
                    mouse_deltas = (x_delta as f32 * MOUSE_SENSITIVITY, y_delta as f32 * MOUSE_SENSITIVITY);
                },
                _ => (),
            }
        }

        integration_parameters.dt = deltasecs;

        physics_pipeline.step(
            &gravity,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut ctx.rigid_body_set,
            &mut ctx.collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            &physics_hooks,
            &event_handler,
        );

        let mut collision_map_list: HashMap<RigidBodyHandle, Vec<RigidBodyHandle>> = HashMap::new();

        while let Ok(collision_event) = collision_recv.try_recv() {
            let (col1_rb, col2_rb) = (
                get_rb_handle_from_collision_handle(&ctx.collider_set, &collision_event.collider1()), 
                get_rb_handle_from_collision_handle(&ctx.collider_set, &collision_event.collider2())
            );
            if let Some(col1_rb) = col1_rb {
                if let Some(col2_rb) = col2_rb {
                    if !collision_map_list.contains_key(&col1_rb) {
                        collision_map_list.insert(col1_rb, vec![]);
                    }
                    if !collision_map_list.contains_key(&col2_rb) {
                        collision_map_list.insert(col2_rb, vec![]);
                    }
                    (*collision_map_list.get_mut(&col1_rb).unwrap()).push(col2_rb);
                    (*collision_map_list.get_mut(&col2_rb).unwrap()).push(col1_rb);
                }
            }
        }

        let should_update_view = camera_controller(&keys_held, mouse_deltas, &mut ctx.camera, 5.0 * deltasecs);
        // let should_update_view = true; 
        update_view_lights = update_view_lights || should_update_view;
        
        /* draw vao verts */

        // if update_view_lights {
        let [v1, v2, v3] = *(ctx.camera.view_pos.as_array());
        let [v4, v5, v6] = *(ctx.camera.light_position.as_array());
        for shader in &ctx.shaders {
            (*shader).set_3_float(UNI_ID[UniEnum::ViewPos as usize], v1, v2, v3);
            (*shader).set_3_float(UNI_ID[UniEnum::LightPos as usize], v4, v5, v6);
            (*shader).set_4_float_matrix(UNI_ID[UniEnum::View as usize], ctx.camera.view_matrix().as_ptr().cast());
        }
            // update_view_lights = false;
        // }

        unsafe { glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT); }

        let mut objs_to_remove = vec![];
        let mut objs_to_add = vec![];

        for obj in ctx.game_obj_store.0.values() {
            let mut obj_bor = obj.borrow_mut();
            let mut loop_ctx = LoopContext{
                go: &mut obj_bor, 
                keys_held: &keys_held, 
                mouse_deltas: &mouse_deltas, 
                camera: &mut ctx.camera, 
                deltasecs, 
                game_time, 
                rigid_body_set: &mut ctx.rigid_body_set, 
                collider_set: &mut ctx.collider_set, 
                floor_set: &mut ctx.floor_set,
                model_map,
                game_obj_store: &ctx.game_obj_store
            };
            loop_ctx.go.physic_update(loop_ctx.rigid_body_set);

            let mut has_rb_and_collisions = false;
            if let Some(obj_rb_handle) = loop_ctx.go.rigid_body_handle {
                if collision_map_list.contains_key(&obj_rb_handle) {
                    has_rb_and_collisions = true;
                }
            }
            if has_rb_and_collisions {
                for rb_handle in collision_map_list.get(&loop_ctx.go.rigid_body_handle.unwrap()).unwrap() {
                    if ctx.game_obj_store.1[rb_handle] == loop_ctx.go.id {
                        // println!("self collide");
                    } else {
                        let other_go_id = ctx.game_obj_store.1[rb_handle];
                        let other = &ctx.game_obj_store.0[&other_go_id];
                        let (mut to_remove, mut to_add) = (ctx.handlecollisionit)(&mut loop_ctx, other);
                        objs_to_remove.append(&mut to_remove);
                        objs_to_add.append(&mut to_add);
                    }
                }
            }

            let (mut to_remove, mut to_add) = (ctx.handleit)(&mut loop_ctx);
            objs_to_remove.append(&mut to_remove);
            objs_to_add.append(&mut to_add);

            let go_model_matrix = obj_bor.model_matrix();
            let go_rotation_matrix = obj_bor.rotation_matrix();

            if let Some( draw ) = &mut obj_bor.drawable_object {

                for drawable in &ctx.drawable_groups[draw.drawable_group_idx].0 {

                    let shader = &ctx.shaders[(*drawable).shader_idx];
                    
                    (*shader).use_program();
                    (*shader).set_4_float_matrix(UNI_ID[UniEnum::Model as usize], (go_model_matrix * draw.model_matrix()).as_ptr().cast());
                    (*shader).set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], (go_rotation_matrix * draw.rotation_matrix()).as_ptr().cast());

                    (ctx.pre_draw)(&shader, &draw);

                    (*drawable).vao.bind();
                    unsafe { glDrawElements(GL_TRIANGLES, (*drawable).tri_count as i32, GL_UNSIGNED_INT, 0 as *const _); }
                }
            }
        }

        for i in objs_to_remove {
            ctx.game_obj_store.remove(&i);
        }

        for obj in objs_to_add {
            ctx.game_obj_store.add(obj);
        }

        /* 2 buffers exist, draw buffer and display buffer
        draw buffer is where the next frame is being built piece by piece
        display buffer is what will be shown on the screen
        swap the draw and display buffer */
        ctx.window.swap_window();

        // while frame_start.elapsed() < target_frame_time {}
        deltatime = frame_start.elapsed();
        if keys_held.contains(&Keycode::F) {
            println!("FPS {:?}", 1.0 / deltatime.as_secs_f32() );
        }
    }
}