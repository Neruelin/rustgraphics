use ultraviolet::{mat, vec};
use ogl33::*;
use beryllium::*;
use std::collections::HashSet;

use crate::gllib::CameraParams;

pub fn camera_controller<'a>(
    keys_held: &'a HashSet<Keycode>,
    mouse_delta: (f32, f32),
    camera: &'a mut CameraParams,
    speed: f32
) -> bool {
    // let front = vec::Vec3::new(f32::cos((*camera).view_rot.x));
    let mut direction = vec::Vec3::zero();
    let mut update_view = false;
    let mut speed_mult = 1;
    if (*keys_held).contains(&Keycode::LSHIFT) { speed_mult = 2;}
    let look_dir = camera.look_dir();
    let look_dir_left = look_dir.cross(vec::Vec3::new(0.0,1.0,0.0));
    if (*keys_held).contains(&Keycode::W) { direction += look_dir; update_view = true;} 
    if (*keys_held).contains(&Keycode::S) { direction -= look_dir; update_view = true;} 
    if (*keys_held).contains(&Keycode::D) { direction += look_dir_left; update_view = true;} 
    if (*keys_held).contains(&Keycode::A) { direction -= look_dir_left; update_view = true;}
    // if (*keys_held).contains(&Keycode::INSERT) { direction.z += -1.0; update_view = true;}
    // if (*keys_held).contains(&Keycode::DELETE) { direction.z += 1.0; update_view = true;}

    if direction != vec::Vec3::zero() { 
        direction.normalize(); 
        direction *= speed * speed_mult as f32;
        (*camera).view_pos += direction;
    }

    if mouse_delta.0.abs() > 0.0 || mouse_delta.1.abs() > 0.0 {
        (*camera).view_rot.y = ((*camera).view_rot.y - mouse_delta.1).clamp(-89.0, 89.0);
        (*camera).view_rot.z = ((*camera).view_rot.z + mouse_delta.0) % 360.0;
        update_view  = true;
    }

    return update_view;
}