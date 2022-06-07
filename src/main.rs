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
use std::{collections::HashSet, io::BufReader, fs::File};
use std::time::{Instant, Duration};
use ultraviolet::{mat,vec,projection};
use tobj;

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
    let mut _rng = rand::thread_rng();

    let sdl = init_sdl();

    let _win = sdl.create_gl_window("OpenGL", WindowPosition::Centered, WINDOW_WIDTH, WINDOW_HEIGHT, WindowFlags::Shown).expect("couldn't make the window");
    // set vsync on to block program until rendered screen has been shown
    _win.set_swap_interval(SwapInterval::Vsync);
       
    unsafe {
        // dynamically load gl functions
        load_gl_with(|f_name| _win.get_proc_address(f_name));

        glEnable(GL_DEPTH_TEST);
    }

    let view_position = vec::Vec3::new(0.0,0.0, 0.0);
    let light_position = vec::Vec3::new(0.0, 10.0, 0.0);
    
    let mut world_translation = mat::Mat4::identity();
    let mut world_rotation = mat::Mat4::identity();
    let model = mat::Mat4::identity();
    let view = mat::Mat4::from_translation(view_position);
    let projection = projection::perspective_gl(45.0_f32, (WINDOW_WIDTH as f32) / (WINDOW_HEIGHT as f32), 0.1, 100.0);


    // const TEXTURE1_FILE_PATH: &str = r#"src/wall.png"#;
    // let mut textures: Vec<Texture> = vec![];
    // const UNIFORM_SHADER_FOLDER: &str = "uniform_shader";
    let mut shaders: Vec<ShaderProgram> = vec![];
    const SHADER_FOLDER_PATH: &str = "src/shaders";
    const BLINN_PHONG_SHADER_FOLDER: &str = "blinn_phong_shader";
    const PARAM_BLINN_PHONG_SHADER_FOLDER: &str = "param_blinn_phong_shader";
    
    // const TEXTURE_SHADER_FOLDER: &str = "texture_shader";
    // textures.push(Texture::from_file(GL_TEXTURE0, TEXTURE1_FILE_PATH, false));
    // shaders.push(texture_program(SHADER_FOLDER_PATH, TEXTURE_SHADER_FOLDER, &(textures[0]), &translation, &rotation, &model, &view, &projection));

    let models_to_load = vec!["src/cone.obj", "src/cube.obj"];
    let mut loaded_models = vec![];

    for model_to_load in models_to_load {
        let (models, _materials) = tobj::load_obj(model_to_load, &tobj::GPU_LOAD_OPTIONS).unwrap();
        let mats = _materials.unwrap();

        let new_shader_idx = shaders.len();
        println!("{:?}", new_shader_idx);
        shaders.push(param_color_program(
            SHADER_FOLDER_PATH, 
            PARAM_BLINN_PHONG_SHADER_FOLDER, 
            mats[0].optical_density,
            &vec::Vec3::from(mats[0].ambient),
            &vec::Vec3::from(mats[0].diffuse),
           &vec::Vec3::from(mats[0].specular),
           mats[0].dissolve,
            &mat::Mat4::identity(), 
            &mat::Mat4::identity(),
            &mat::Mat4::identity(),
            &view,
            &projection
        ));

        loaded_models.push((
            combine_loaded_data( &models[0]), 
            models[0].mesh.indices.clone(), 
            new_shader_idx
        ));
    }

    let mut meshes: Vec<MeshData> = vec![];

    for model in loaded_models {
        let tris = model.1.len();
        meshes.push(MeshData(model.0, model.1, tris, model.2));
    }

    let mut drawable_objs: Vec<Drawable> = vec![];

    for mesh in meshes {
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
                buffer_data(BufferType::Array, bytemuck::cast_slice(mesh.0.as_slice()), GL_STATIC_DRAW);
                vbo
            };

            /* generate buffer to hold groups of vertexes that form triangles
            set as the active element array buffer type
            load in the data */
            let ebo = {
                let ebo = Buffer::new().expect("Couldn't make a new buffer");
                ebo.bind(BufferType::ElementArray);
                buffer_data(BufferType::ElementArray, bytemuck::cast_slice(mesh.1.as_slice()), GL_STATIC_DRAW);
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
        drawable_objs.push(Drawable(vao, vbo, ebo, mesh.2, mesh.3));
    }

    
    
    /* a complete graphics pipeline combines a vertex and fragment shader
    create a new shader program object */

    let my_shader_idx = shaders.len();
    shaders.push(color_program(SHADER_FOLDER_PATH, BLINN_PHONG_SHADER_FOLDER, &vec::Vec3::new(1.0, 1.0, 0.0), &world_translation, &world_rotation, &model, &view, &projection));

    println!("Shaders: {:?}", shaders.len());

    println!("{:?}", drawable_objs[0].4);

    let objs_to_draw: Vec<DrawableObject> = vec![
        DrawableObject(vec::Vec3::new( 0.0, 0.0, -10.0), 0, drawable_objs[0].4),
        DrawableObject(vec::Vec3::new( 2.0, 0.0, -10.0), 0, my_shader_idx),
        DrawableObject(vec::Vec3::new( -2.0, 0.0, -10.0), 1, drawable_objs[1].4),
        DrawableObject(vec::Vec3::new( 4.0, 0.0, -10.0), 1, my_shader_idx),
    ];

    /* Keyboard input storage */
    let mut keys_held = HashSet::new();
    let mut input_translation = mat::Mat4::from_translation(vec::Vec3::zero());

    /* Time and FPS configuration */
    let mut deltatime = Duration::new(0, 0);
    let target_fps: f32 = 60.0;
    let target_frame_micros = (1000000_f32 / target_fps).ceil() as u64;
    let target_frame_time = Duration::from_micros(target_frame_micros);
    let _start_instant = Instant::now();
    let mut update_view_lights_worldtrans = true;

    clear_color(0.0, 0.0, 0.0, 1.0);

    let (mut rx, mut ry) = (0_f32, 0_f32);

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

        if keys_held.contains(&Keycode::UP) { rx += 1.0 * deltasecs; } 
        if keys_held.contains(&Keycode::DOWN) { rx -= 1.0 * deltasecs; } 
        if keys_held.contains(&Keycode::RIGHT) { ry += 1.0 * deltasecs; }
        if keys_held.contains(&Keycode::LEFT) { ry -= 1.0 * deltasecs; }

        let model_rotation = mat::Mat4::from_euler_angles(rx, ry, 0.0);

        let mut direction = vec::Vec3::zero();

        if keys_held.contains(&Keycode::W) { direction.y += 1.0; } 
        if keys_held.contains(&Keycode::S) { direction.y += -1.0; } 
        if keys_held.contains(&Keycode::D) { direction.x += 1.0; } 
        if keys_held.contains(&Keycode::A) { direction.x += -1.0; }
        if keys_held.contains(&Keycode::INSERT) { direction.z += -1.0; }
        if keys_held.contains(&Keycode::DELETE) { direction.z += 1.0; }

        if direction != vec::Vec3::zero() { 
            direction.normalize(); 
        }

        direction *= 0.5 * deltasecs;

        input_translation.translate(&direction);

        /* draw vao verts */

        if update_view_lights_worldtrans {
            println!("update view lights and world translation");
            let [v1, v2, v3] = *(view_position.as_array());
            let [v4, v5, v6] = *(light_position.as_array());
            for shader in &shaders {
                (*shader).set_4_float_matrix(UNI_ID[UniEnum::Translation as usize], world_translation.as_ptr().cast());
                (*shader).set_3_float(UNI_ID[UniEnum::ViewPos as usize], v1, v2, v3);
                (*shader).set_3_float(UNI_ID[UniEnum::LightPos as usize], v4, v5, v6);
            }
            update_view_lights_worldtrans = false;
        }

        for obj in &objs_to_draw {
            let (model_translation, model_type, shader_index) = ((*obj).0, (*obj).1, (*obj).2);
            let shader = &shaders[shader_index];
            let mut temp_model = mat::Mat4::identity();
            temp_model = mat::Mat4::from_translation(model_translation) * input_translation * model_rotation * temp_model;

            (*shader).use_program();
            (*shader).set_4_float_matrix(UNI_ID[UniEnum::Model as usize], temp_model.as_ptr().cast());
            (*shader).set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], model_rotation.as_ptr().cast());
            let vao = &drawable_objs[model_type].0;
            (*vao).bind();
            unsafe { glDrawElements(GL_TRIANGLES, drawable_objs[model_type].3 as i32, GL_UNSIGNED_INT, 0 as *const _); }
        }


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
