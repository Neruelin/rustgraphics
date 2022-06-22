mod gllib;
mod camera;
mod scenes;
mod behaviors;

use std::collections::HashMap;

use crate::behaviors::apply_behaviors;
use crate::behaviors::apply_collision_behaviors;
use crate::behaviors::BehaviorDataContainerEnum;
use crate::gllib::*;
use crate::scenes::*;

/* Takes a string literal and concatenates a null byte onto the end. */
#[macro_export]
macro_rules! null_str {
  ($lit:literal) => {{
    // "type check" the input
    const _: &str = $lit;
    concat!($lit, "\0")
  }};
}

fn main() {
    
    const WINDOW_WIDTH: u32 = 800;
    const WINDOW_HEIGHT: u32 = 600;

    let mut ctx: Context<BehaviorDataContainerEnum> = Context::new(WINDOW_WIDTH, WINDOW_HEIGHT).expect("creating window failed probably");

    let mut model_map: HashMap<&str, usize> = HashMap::new();

    model_map.insert("cone", ctx.load_model("src/models/cone.obj"));
    model_map.insert("cube", ctx.load_model("src/models/cube.obj"));
    model_map.insert("cone_ring", ctx.load_model("src/models/cone_ring.obj"));
    model_map.insert("plane", ctx.load_model("src/models/plane.obj"));
    model_map.insert("ball", ctx.load_model("src/models/ball.obj"));

    // let scene_name = "";
    // let scene_name = "waves";
    let scene_name = "physics";

    ctx.pre_draw = {
        match scene_name {
            "waves" => {
                make_scene_waves(&mut ctx, &model_map)
            },
            "physics" => {
                make_scene_physics(&mut ctx, &model_map)
            },
            _ => make_scene_empty(&mut ctx, &model_map)
        }
    };
    ctx.handleit = Box::new(apply_behaviors);
    ctx.handlecollisionit = Box::new(apply_collision_behaviors);

    main_loop(&mut ctx, &model_map);

}
