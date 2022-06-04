#![allow(unused_imports)]

use beryllium::*;
use ogl33::*;
use core::{
    convert::{TryFrom, TryInto},
    mem::{size_of, size_of_val},
    ptr::null,
};
use image::io::Reader as ImageReader;
use rand::Rng;
use std::collections::HashSet;
use std::time::{Instant, Duration};
use ultraviolet::{mat,vec,projection};

mod gllib;
use crate::gllib::*;

/// Takes a string literal and concatenates a null byte onto the end.
#[macro_export]
macro_rules! null_str {
  ($lit:literal) => {{
    // "type check" the input
    const _: &str = $lit;
    concat!($lit, "\0")
  }};
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

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

fn main() {
    let mut rng = rand::thread_rng();

    let sdl = init_sdl();

    let _win = sdl.create_gl_window("OpenGL", WindowPosition::Centered, WINDOW_WIDTH, WINDOW_HEIGHT, WindowFlags::Shown).expect("couldn't make the window");
    // set vsync on to block program until rendered screen has been shown
    _win.set_swap_interval(SwapInterval::Vsync);

    // 3d position coords, rgb values, 2d texel coords
    type Vertex = [f32; 3 + 2];
    type TriIndexes = [u32; 3];

    let mut transform = mat::Mat4::identity();
    transform.translate(&vec::Vec3::new(0.0,0.0,-2.0));
    // transform.cols[3] += vec::Vec4::new(1.0,0.0,0.0,0.0);
    // println!("{:?}", transform);

    let vertices: [Vertex; 36] = [
        [-0.5, -0.5, -0.5, 0.0, 0.0],
        [0.5, -0.5, -0.5, 1.0, 0.0],
        [0.5, 0.5, -0.5, 1.0, 1.0],
        [0.5, 0.5, -0.5, 1.0, 1.0],
        [-0.5, 0.5, -0.5, 0.0, 1.0],
        [-0.5, -0.5, -0.5, 0.0, 0.0],
        [-0.5, -0.5, 0.5, 0.0, 0.0],
        [0.5, -0.5, 0.5, 1.0, 0.0],
        [0.5, 0.5, 0.5, 1.0, 1.0],
        [0.5, 0.5, 0.5, 1.0, 1.0],
        [-0.5, 0.5, 0.5, 0.0, 1.0],
        [-0.5, -0.5, 0.5, 0.0, 0.0],
        [-0.5, 0.5, 0.5, 1.0, 0.0],
        [-0.5, 0.5, -0.5, 1.0, 1.0],
        [-0.5, -0.5, -0.5, 0.0, 1.0],
        [-0.5, -0.5, -0.5, 0.0, 1.0],
        [-0.5, -0.5, 0.5, 0.0, 0.0],
        [-0.5, 0.5, 0.5, 1.0, 0.0],
        [0.5, 0.5, 0.5, 1.0, 0.0],
        [0.5, 0.5, -0.5, 1.0, 1.0],
        [0.5, -0.5, -0.5, 0.0, 1.0],
        [0.5, -0.5, -0.5, 0.0, 1.0],
        [0.5, -0.5, 0.5, 0.0, 0.0],
        [0.5, 0.5, 0.5, 1.0, 0.0],
        [-0.5, -0.5, -0.5, 0.0, 1.0],
        [0.5, -0.5, -0.5, 1.0, 1.0],
        [0.5, -0.5, 0.5, 1.0, 0.0],
        [0.5, -0.5, 0.5, 1.0, 0.0],
        [-0.5, -0.5, 0.5, 0.0, 0.0],
        [-0.5, -0.5, -0.5, 0.0, 1.0],
        [-0.5, 0.5, -0.5, 0.0, 1.0],
        [0.5, 0.5, -0.5, 1.0, 1.0],
        [0.5, 0.5, 0.5, 1.0, 0.0],
        [0.5, 0.5, 0.5, 1.0, 0.0],
        [-0.5, 0.5, 0.5, 0.0, 0.0],
        [-0.5, 0.5, -0.5, 0.0, 1.0]
    ];
    
    let cubes: [vec::Vec3; 10] = [
        vec::Vec3::new( 0.0, 0.0, 0.0),
        vec::Vec3::new( 2.0, 5.0, -15.0),
        vec::Vec3::new(-1.5, -2.2, -2.5),
        vec::Vec3::new(-3.8, -2.0, -12.3),
        vec::Vec3::new( 2.4, -0.4, -3.5),
        vec::Vec3::new(-1.7, 3.0, -7.5),
        vec::Vec3::new( 1.3, -2.0, -2.5),
        vec::Vec3::new( 1.5, 2.0, -2.5),
        vec::Vec3::new( 1.5, 0.2, -1.5),
        vec::Vec3::new(-1.3, 1.0, -1.5)
    ];
       
    const INDICES: [TriIndexes; 2] = [[0,1,3],[1,2,3]];

    unsafe {
        // dynamically load gl functions
        load_gl_with(|f_name| _win.get_proc_address(f_name));

        glEnable(GL_DEPTH_TEST);
    }

    clear_color(0.0, 0.0, 0.0, 1.0);

    /* generate a Vertex Array Object and store ref in mutable vao variable
    and bind the VAO making it the active */
    let vao = VertexArray::new().expect("Couldn't make a new VAO");
    vao.bind();
    
    /* generate a Buffer Object and store req in mutable vbo variable
    and set the given buffer as the current active Array Buffer 
    and then initializes target's active buffer's storage with a size and initial data and a hint to its usage
    and then initialize active Array Buffer with size of vertex array and pointer to vertex array */
    let vbo = Buffer::new().expect("Couldn't make a new buffer");
    vbo.bind(BufferType::Array);
    buffer_data(BufferType::Array, bytemuck::cast_slice(&vertices), GL_STATIC_DRAW);

    /* generate buffer to hold groups of vertexes that form triangles
    set as the active element array buffer type
    load in the data */
    let ebo = Buffer::new().expect("Couldn't make a new buffer");
    ebo.bind(BufferType::ElementArray);
    buffer_data(BufferType::ElementArray, bytemuck::cast_slice(&INDICES), GL_STATIC_DRAW);
    
    unsafe {

        /* create vertex attribute indexed at 0 that describes how the vertex data is represented in the buffer
        vertex shader program can find this attribute by index
        and enables the Vertex Attribute Pointer */
        // vertex
        glVertexAttribPointer(
            0,
            3,
            GL_FLOAT,
            GL_FALSE,
            size_of::<Vertex>().try_into().unwrap(),
            0 as *const _,
        );
        glEnableVertexAttribArray(0);

        // color
        // glVertexAttribPointer(
        //     1, 
        //     3, 
        //     GL_FLOAT, 
        //     GL_FALSE, 
        //     size_of::<Vertex>().try_into().unwrap(), 
        //     (size_of::<f32>() * 3) as *const _,
        // );
        // glEnableVertexAttribArray(1);

        // third attribute for texture
        glVertexAttribPointer(
            1,
            2,
            GL_FLOAT,
            GL_FALSE,
            size_of::<Vertex>().try_into().unwrap(),
            size_of::<[f32; 3]>() as *const _,
        );
        glEnableVertexAttribArray(1);
    }

    let model = mat::Mat4::identity(); // mat::Mat4::from_rotation_x(0.0) * mat::Mat4::from_rotation_y(0.0) * mat::Mat4::from_rotation_z(0.0);
    let view = mat::Mat4::from_translation(vec::Vec3::new(0.0,0.0,-1.0));
    let projection = projection::perspective_gl(45.0_f32, (WINDOW_WIDTH as f32) / (WINDOW_HEIGHT as f32), 0.1, 100.0);

    let model_uniform_name = "model\0";
    let view_uniform_name = "view\0";
    let projection_uniform_name = "projection\0";

    /* a complete graphics pipeline combines a vertex and fragment shader
    create a new shader program object */
    const SHADER_FOLDER_PATH: &str = "src/shaders";
    let transform_uniform_name = "transform\0";

    const EXAMPLE_SHADER_FOLDER: &str = "example_shader";
    let colored_shader_program = {
        let vert = format!("{}/{}/{}", SHADER_FOLDER_PATH, EXAMPLE_SHADER_FOLDER, "vertex");
        let frag = format!("{}/{}/{}", SHADER_FOLDER_PATH, EXAMPLE_SHADER_FOLDER, "fragment");
        let shader = ShaderProgram::from_files(&vert, &frag).unwrap();
        shader.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        shader.set_4_float_matrix(model_uniform_name, model.as_ptr().cast());
        shader.set_4_float_matrix(view_uniform_name, view.as_ptr().cast());
        shader.set_4_float_matrix(projection_uniform_name, projection.as_ptr().cast());
        shader
    };
    
    const UNIFORM_SHADER_FOLDER: &str = "uniform_shader";
    let vertex_uniform_name = "our_color\0";
    let uniform_colored_shader_program = {
        let vert = format!("{}/{}/{}", SHADER_FOLDER_PATH, UNIFORM_SHADER_FOLDER, "vertex");
        let frag = format!("{}/{}/{}", SHADER_FOLDER_PATH, UNIFORM_SHADER_FOLDER, "fragment");
        let shader = ShaderProgram::from_files(&vert, &frag).unwrap();
        shader.set_4_float(
            vertex_uniform_name,
            0.0, 1.0, 0.0, 0.0
        );
        shader.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        shader.set_4_float_matrix(model_uniform_name, model.as_ptr().cast());
        shader.set_4_float_matrix(view_uniform_name, view.as_ptr().cast());
        shader.set_4_float_matrix(projection_uniform_name, projection.as_ptr().cast());
        shader
    };
    
    const TEXTURE_SHADER_FOLDER: &str = "texture_shader";
    const TEXTURE1_FILE_PATH: &str = r#"src/wall.png"#;
    let texture_uniform_name = "our_texture\0";
    let texture1 = Texture::from_file(GL_TEXTURE0, TEXTURE1_FILE_PATH, false);
    let texture1_colored_shader_program= texture_program( SHADER_FOLDER_PATH, TEXTURE_SHADER_FOLDER, &texture1, &transform, &model, &view, &projection);

    
    const TEXTURE2_FILE_PATH: &str = r#"src/container.jpg"#;
    let texture2 = Texture::from_file(GL_TEXTURE1, TEXTURE2_FILE_PATH, false);
    let texture2_colored_shader_program = {
        let vert = format!("{}/{}/{}", SHADER_FOLDER_PATH, TEXTURE_SHADER_FOLDER, "vertex");
        let frag = format!("{}/{}/{}", SHADER_FOLDER_PATH, TEXTURE_SHADER_FOLDER, "fragment");
        let shader = ShaderProgram::from_files_with_texture(
            &vert, 
            &frag,
            &texture2,
            texture_uniform_name
        ).unwrap();
        shader.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        shader.set_4_float_matrix(model_uniform_name, model.as_ptr().cast());
        shader.set_4_float_matrix(view_uniform_name, view.as_ptr().cast());
        shader.set_4_float_matrix(projection_uniform_name, projection.as_ptr().cast());
        shader
    };

    const TWO_TEXTURE_SHADER_FOLDER: &str = "two_texture_shader";
    let vert = format!("{}/{}/{}", SHADER_FOLDER_PATH, TWO_TEXTURE_SHADER_FOLDER, "vertex");
    let frag = format!("{}/{}/{}", SHADER_FOLDER_PATH, TWO_TEXTURE_SHADER_FOLDER, "fragment");

    let texture3_uniform_name = "our_texture\0";
    const TEXTURE3_FILE_PATH: &str = r#"src/wall.png"#;
    let texture4_uniform_name = "our_texture2\0";
    const TEXTURE4_FILE_PATH: &str = r#"src/awesomeface.png"#;
    let mix_level_uniform_name = "mix_level\0";
    let texture3 = Texture::from_file(GL_TEXTURE3, TEXTURE3_FILE_PATH, false);
    let texture4 = Texture::from_file(GL_TEXTURE4, TEXTURE4_FILE_PATH, true);
    let two_texture_colored_shader_program = {
        let shader = ShaderProgram::from_files_with_texture(
            &vert, 
            &frag,
            &texture3,
            texture3_uniform_name
        ).unwrap();
        shader.set_int_bool(texture4_uniform_name, texture4.texture_uniform_id());
        shader.set_1_float(mix_level_uniform_name, 0.5_f32);
        shader.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        shader.set_4_float_matrix(model_uniform_name, model.as_ptr().cast());
        shader.set_4_float_matrix(view_uniform_name, view.as_ptr().cast());
        shader.set_4_float_matrix(projection_uniform_name, projection.as_ptr().cast());
        shader
    };

    /* Keyboard input storage */
    let mut keys_held = HashSet::new();

    /* Time and FPS configuration */
    let mut deltatime = Duration::new(0, 0);
    let target_fps: f32 = 60.0;
    let target_frame_micros = (1000000_f32 / target_fps).ceil() as u64;
    let target_frame_time = Duration::from_micros(target_frame_micros);
    let start_instant = Instant::now();

    let mut mix_level = 0.5_f32;

    'main_loop: loop {
        let frame_start = Instant::now();
        let deltasecs = deltatime.as_secs_f32();

        unsafe { glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT); }

        while let Some(event) = sdl.poll_events().and_then(Result::ok) {
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
                _ => (),
            }
        }

        clear_color(0.0,0.0,0.0,1.0);
        
        if keys_held.contains(&Keycode::PAGEUP) {
            mix_level = mix_level + (0.5 * deltasecs)
        } else if keys_held.contains(&Keycode::PAGEDOWN) {
            mix_level = mix_level - (0.5 * deltasecs)
        }

        if mix_level > 1.0 {
            mix_level = 1.0_f32;
        } else if mix_level < 0.0 {
            mix_level = 0.0_f32;
        }

        let mut rotation = mat::Mat4::identity();

        if keys_held.contains(&Keycode::UP) {
            rotation = rotation * mat::Mat4::from_euler_angles(1.0 * deltasecs, 0.0, 0.0);
        } 
        if keys_held.contains(&Keycode::DOWN) {
            rotation = rotation * mat::Mat4::from_euler_angles(-1.0 * deltasecs, 0.0, 0.0);
        } 
        if keys_held.contains(&Keycode::RIGHT) {
            rotation = rotation * mat::Mat4::from_euler_angles(0.0, 1.0 * deltasecs, 0.0);
        }
        if keys_held.contains(&Keycode::LEFT) {
            rotation = rotation * mat::Mat4::from_euler_angles(0.0, -1.0 * deltasecs, 0.0);
        }

        let mut direction = vec::Vec3::zero();

        if keys_held.contains(&Keycode::W) {
            direction.y += 1.0;
        } 
        if keys_held.contains(&Keycode::S) {
            direction.y += -1.0;
        } 
        if keys_held.contains(&Keycode::D) {
            direction.x += 1.0;
        } 
        if keys_held.contains(&Keycode::A) {
            direction.x += -1.0;
        }
        if keys_held.contains(&Keycode::INSERT) {
            direction.z += -1.0;
        }
        if keys_held.contains(&Keycode::DELETE) {
            direction.z += 1.0;
        }

        if direction != vec::Vec3::zero() {
            direction.normalize();
        }

        direction *= 0.5 * deltasecs;

        transform.translate(&direction);
        transform = transform * rotation;

        colored_shader_program.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        uniform_colored_shader_program.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        texture1_colored_shader_program.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        texture2_colored_shader_program.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());
        two_texture_colored_shader_program.set_4_float_matrix(transform_uniform_name, transform.as_ptr().cast());

        let blue_val = 0.5 + ((start_instant.elapsed().as_millis() as f32 / 1000_f32).sin() / 2_f32);
        let green_val = 0.5 + ((start_instant.elapsed().as_millis() as f32 / 1000_f32).cos() / 2_f32);
        uniform_colored_shader_program.set_4_float(vertex_uniform_name, 0.0, blue_val, green_val, 0.0);

        let mut active_shader: &ShaderProgram = &uniform_colored_shader_program;

        /* choose shader to use on draw */
        if keys_held.contains(&Keycode::_2) {
            active_shader = &colored_shader_program;
        } else if keys_held.contains(&Keycode::_3) {
            active_shader = &texture1_colored_shader_program;
        } else if keys_held.contains(&Keycode::_4) {
            active_shader = &texture2_colored_shader_program;
        } else if keys_held.contains(&Keycode::_5) {
            two_texture_colored_shader_program.set_1_float(mix_level_uniform_name, mix_level);
            active_shader = &two_texture_colored_shader_program;
        }

        /* draw from vertices in current active vertex buffer object
        specifies the first vertex to draw and how many should be drawn
        the current active vertex buffer object must contain enough verticies give the offset and count otherwise the gpu will segfault
        glDrawArrays(GL_TRIANGLES, 0, 3);
        instead draw elements when using element array buffers to make more triangles from common vertices
        draw in triangle mode
        there are 6 elements in the element array buffer
        provide data type of element array buffer 
        provide a pointer offset to start rendering further in the buffer in this case no offset to begin at the start */
        /* choose vao to use on draw */
        vao.bind();
        
        /* draw vao verts */
        
        for i in 1..10 {
            let mut temp_model = mat::Mat4::identity();
            temp_model.translate(&cubes[i]);
            colored_shader_program.set_4_float_matrix(model_uniform_name, temp_model.as_ptr().cast());
            uniform_colored_shader_program.set_4_float_matrix(model_uniform_name, temp_model.as_ptr().cast());
            texture1_colored_shader_program.set_4_float_matrix(model_uniform_name, temp_model.as_ptr().cast());
            texture2_colored_shader_program.set_4_float_matrix(model_uniform_name, temp_model.as_ptr().cast());
            two_texture_colored_shader_program.set_4_float_matrix(model_uniform_name, temp_model.as_ptr().cast());

            active_shader.use_program();

            unsafe { glDrawArrays(GL_TRIANGLES, 0, 36); }
        }
        // unsafe { glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_INT, 0 as *const _); }

        /* 2 buffers exist, draw buffer and display buffer
        draw buffer is where the next frame is being built piece by piece
        display buffer is what will be shown on the screen
        swap the draw and display buffer */
        _win.swap_window();

        while frame_start.elapsed() < target_frame_time {}
        deltatime = frame_start.elapsed();
        if keys_held.contains(&Keycode::F) {
            println!("frametime {:?}", deltatime.as_micros());
        }
    }
}
