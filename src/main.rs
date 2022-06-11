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
mod camera;
use crate::camera::*;

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
const SHADER_FOLDER_PATH: &str = "src/shaders";
const BLINN_PHONG_SHADER_FOLDER: &str = "blinn_phong_shader";
const PARAM_BLINN_PHONG_SHADER_FOLDER: &str = "param_blinn_phong_shader";

fn main() {
    let mut rng = rand::thread_rng();

    let sdl = init_sdl();

    let _win = sdl.create_gl_window("OpenGL", WindowPosition::Centered, WINDOW_WIDTH, WINDOW_HEIGHT, WindowFlags::Shown).expect("couldn't make the window");
    // set vsync on to block program until rendered screen has been shown
    _win.set_swap_interval(SwapInterval::Vsync);
       
    unsafe {
        // dynamically load gl functions
        load_gl_with(|f_name| _win.get_proc_address(f_name));

        glEnable(GL_DEPTH_TEST);
    }

    let light_position = vec::Vec3::new(0.0, 10.0, -10.0);
    let default_model = mat::Mat4::identity();
        
    let mut camera = CameraParams::new(
        vec::Vec3::zero(),
        vec::Vec3::new(0.0, 0.0, 90.0),
        projection::perspective_gl(45.0_f32, (WINDOW_WIDTH as f32) / (WINDOW_HEIGHT as f32), 0.1, 100.0)
        );

    let mut shaders: Vec<ShaderProgram> = vec![];
    
    let models_to_load = vec!["src/cone.obj", "src/cube.obj"];
    let mut meshes: Vec<MeshData> = vec![];

    for model_to_load in models_to_load {
        let (models, _materials) = tobj::load_obj(model_to_load, &tobj::GPU_LOAD_OPTIONS).unwrap();
        let mats = _materials.unwrap();

        let new_shader_idx = shaders.len();
        shaders.push(param_color_program(
            SHADER_FOLDER_PATH, 
            PARAM_BLINN_PHONG_SHADER_FOLDER, 
            mats[0].optical_density,
            &vec::Vec3::from(mats[0].ambient),
            &vec::Vec3::from(mats[0].diffuse),
           &vec::Vec3::from(mats[0].specular),
            mats[0].dissolve,
            &default_model,
            &camera.view_matrix(),
            &camera.projection
        ));

        let tris = models[0].mesh.indices.len();

        meshes.push(MeshData(
            combine_loaded_data( &models[0]), 
            models[0].mesh.indices.clone(), 
            tris,
            new_shader_idx
        ));
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

    let _my_shader_idx = shaders.len();
    shaders.push(color_program(
        SHADER_FOLDER_PATH, 
        BLINN_PHONG_SHADER_FOLDER, 
        &vec::Vec3::new(0.0, 1.0, 0.0), 
        &default_model, 
        &camera.view_matrix(),
        &camera.projection
        ));

    let mut objs_to_draw: Vec<GameObject> = vec![];

    // for _ in 0..10000 {
    //     objs_to_draw.push(make_go(vec::Vec3::new( rng.gen_range(0_f32, 10_f32), rng.gen_range(0_f32, 10_f32), -30.0), 0, drawable_objs[0].4));
    // }
    
    for x in -10..10 {
        for y in -10..10 {
            let x_off = if ((x % 2) == 0) != ((y % 2) == 0) {1} else {0};
            // let y = if y % 2 == 0 {y + 1} else {y};
            objs_to_draw.push(make_go(vec::Vec3::new( ((2*x)) as f32, -2.0 - x_off as f32, (2*y) as f32), 1, drawable_objs[1].4));
        }
    }

    objs_to_draw.push(make_go(vec::Vec3::new( 20 as f32, 0.0, 20 as f32), 1, drawable_objs[1].4));

    
    objs_to_draw[0].behaviors.push(Box::new(WASDish(2)));


    /* mouse input config */
    const MOUSE_SENSITIVITY: f32 = 0.4;
    sdl.set_relative_mouse_mode(true).unwrap();

    /* Keyboard input storage */
    let mut keys_held = HashSet::new();

    /* Time and FPS configuration */
    let mut deltatime = Duration::new(0, 0);
    let target_fps: f32 = 60.0;
    let target_frame_micros = (1000000_f32 / target_fps).ceil() as u64;
    let target_frame_time = Duration::from_micros(target_frame_micros);
    let start_instant = Instant::now();
    let mut update_view_lights = true;

    clear_color(0.0, 0.0, 0.0, 1.0);

    let (mut rx, mut ry, mut rz) = (0_f32, 0_f32, 0_f32);

    'main_loop: loop {
        let frame_start = Instant::now();
        let deltasecs = deltatime.as_secs_f32();
        let game_time = start_instant.elapsed().as_secs_f32();
        let mut mouse_delta = (0.0, 0.0);

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
                Event::MouseMotion(MouseMotionEvent { x_delta, y_delta, .. }) => {
                    mouse_delta = (x_delta as f32 * MOUSE_SENSITIVITY, y_delta as f32 * MOUSE_SENSITIVITY);
                },
                _ => (),
            }
        }

        clear_color(0.0,0.0,0.0,1.0);

        if keys_held.contains(&Keycode::UP) { rx += 1.0 * deltasecs; } 
        if keys_held.contains(&Keycode::DOWN) { rx -= 1.0 * deltasecs; } 
        if keys_held.contains(&Keycode::RIGHT) { ry += 1.0 * deltasecs; }
        if keys_held.contains(&Keycode::LEFT) { ry -= 1.0 * deltasecs; }
        if keys_held.contains(&Keycode::PAGEUP) { rz += 1.0 * deltasecs; }
        if keys_held.contains(&Keycode::PAGEDOWN) { rz -= 1.0 * deltasecs; }

        let should_update_view = camera_controller(&keys_held, mouse_delta, &mut camera, 5.0 * deltasecs);
        update_view_lights = update_view_lights || should_update_view;
        
        /* draw vao verts */

        if update_view_lights {
            let [v1, v2, v3] = *(camera.view_pos.as_array());
            let [v4, v5, v6] = *(light_position.as_array());
            for shader in &shaders {
                (*shader).set_3_float(UNI_ID[UniEnum::ViewPos as usize], v1, v2, v3);
                (*shader).set_3_float(UNI_ID[UniEnum::LightPos as usize], v4, v5, v6);
                (*shader).set_4_float_matrix(UNI_ID[UniEnum::View as usize], camera.view_matrix().as_ptr().cast());
            }
            update_view_lights = false;
        }

        for obj in &mut objs_to_draw {
            
            obj.do_actions(&keys_held, deltasecs, game_time);

            if let Some( draw ) = &mut obj.drawable_object {

                (*draw).rotation.x = rx;
                (*draw).rotation.y = ry;
                (*draw).rotation.z = rz;

                let shader = &shaders[(*draw).shader_idx];
                
                (*shader).use_program();
                (*shader).set_4_float_matrix(UNI_ID[UniEnum::Model as usize], (*draw).model_matrix().as_ptr().cast());
                (*shader).set_4_float_matrix(UNI_ID[UniEnum::Rotation as usize], (*draw).rotation_matrix().as_ptr().cast());
                (*shader).set_3_float(UNI_ID[UniEnum::DiffuseColor as usize], (*draw).position.x / 10.0 + 1.0, (*draw).position.y + 3.0, (*draw).position.z / 10.0 + 1.0);
                let mesh = &drawable_objs[(*draw).mesh_idx];
                (*mesh).0.bind();
                unsafe { glDrawElements(GL_TRIANGLES, (*mesh).3 as i32, GL_UNSIGNED_INT, 0 as *const _); }
            }
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
