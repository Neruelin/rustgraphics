#![allow(unused_variables, dead_code)]
use crate::gllib::*;
use rapier2d::prelude::*;
use beryllium::*;
use std::{cell::{RefCell}, time::{Instant, Duration}};
use ultraviolet::vec;

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum Behaviors {
    BArrowControl,
    BDebuggin,
    BCameraTracking,
    BSpawnBall,
    BAttractionTo,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]

pub enum BehaviorData {
    Behaviors(Behaviors),
}

pub trait BehaviorDataContainer {}

pub fn apply_behaviors(
    loop_ctx: &mut LoopContext<BehaviorDataContainerEnum>,
) -> (Vec<GameObjectID>, Vec<GameObject<BehaviorDataContainerEnum>>) {
    let mut behaviors_copy = vec![];
    for b in loop_ctx.go.behaviors.iter() {
        behaviors_copy.push(*b);
    }
    let mut objs_to_remove = vec![];
    let mut objs_to_add = vec![];
    for b in behaviors_copy {
        match b {
            Behaviors::BArrowControl => {
                let (mut to_remove, mut to_add) = arrow_control(loop_ctx);
                objs_to_remove.append(&mut to_remove);
                objs_to_add.append(&mut to_add);
            },
            Behaviors::BDebuggin => {
                let (mut to_remove, mut to_add) = debuggin(loop_ctx);
                objs_to_remove.append(&mut to_remove);
                objs_to_add.append(&mut to_add);
            },
            Behaviors::BCameraTracking => {
                let (mut to_remove, mut to_add) = camera_tracking(loop_ctx);
                objs_to_remove.append(&mut to_remove);
                objs_to_add.append(&mut to_add);
            },
            Behaviors::BSpawnBall => {
                let (mut to_remove, mut to_add) = spawn_ball(loop_ctx);
                objs_to_remove.append(&mut to_remove);
                objs_to_add.append(&mut to_add);
            },
            Behaviors::BAttractionTo => {
                let (mut to_remove, mut to_add) = attraction_to(loop_ctx);
                objs_to_remove.append(&mut to_remove);
                objs_to_add.append(&mut to_add);
            }
        }
    }
    (objs_to_remove, objs_to_add)
}

pub fn arrow_control(
    loop_ctx: &mut LoopContext<BehaviorDataContainerEnum>
) -> (Vec<GameObjectID>, Vec<GameObject<BehaviorDataContainerEnum>>) {
    if let Some(rb_handle) = loop_ctx.go.rigid_body_handle {
        if let Some(BehaviorDataContainerEnum::ArrowControlData(arrow_control_data)) = loop_ctx.go.behaviors_data.get(&BehaviorData::Behaviors(Behaviors::BArrowControl)) {
            let mut impulse = vector![0.0,0.0];
            let mut moved = false;  
            if loop_ctx.keys_held.contains(&Keycode::RIGHT) { impulse.x += arrow_control_data.accel ; moved = true;}
            if loop_ctx.keys_held.contains(&Keycode::LEFT) { impulse.x -= arrow_control_data.accel ; moved = true;}
            if loop_ctx.go.grounded && loop_ctx.keys_held.contains(&Keycode::SPACE) { impulse.y = arrow_control_data.accel * 2.0; loop_ctx.go.grounded = false;}
            loop_ctx.rigid_body_set[rb_handle].apply_impulse(impulse, true);
            let mut linvel = loop_ctx.rigid_body_set[rb_handle].linvel().clone();
            if loop_ctx.go.grounded && !moved {
                linvel.x = linvel.x * 0.95;
            } 
            linvel.x = f32::max(f32::min(linvel.x, arrow_control_data.max_speed), -arrow_control_data.max_speed);
            loop_ctx.rigid_body_set[rb_handle].set_linvel(linvel, true);
        }
    }
    (vec![], vec![])
}
#[derive(Debug)]

pub struct ArrowControlData{
    pub accel: f32,
    pub max_speed: f32
}
impl BehaviorDataContainer for ArrowControlData{}

pub fn debuggin(
    loop_ctx: &mut LoopContext<BehaviorDataContainerEnum>
) -> (Vec<GameObjectID>, Vec<GameObject<BehaviorDataContainerEnum>>) {
    let (mut rx, mut ry, rz) = (0.0_f32, 0.0_f32, 0.0_f32);
    if loop_ctx.keys_held.contains(&Keycode::I) { ry += 1.0; } 
    if loop_ctx.keys_held.contains(&Keycode::K) { ry -= 1.0; } 
    if loop_ctx.keys_held.contains(&Keycode::L) { rx += 1.0; }
    if loop_ctx.keys_held.contains(&Keycode::J) { rx -= 1.0; }
    if let Some(draw_obj) = &mut loop_ctx.go.drawable_object {
        if loop_ctx.keys_held.contains(&Keycode::SPACE) { println!("{:?}", draw_obj.position) }
        draw_obj.position.x += rx * loop_ctx.deltasecs;
        draw_obj.position.y += ry * loop_ctx.deltasecs;
        draw_obj.position.z += rz * loop_ctx.deltasecs;
    }
    (vec![], vec![])
}

pub fn camera_tracking(
    loop_ctx: &mut LoopContext<BehaviorDataContainerEnum>
) -> (Vec<GameObjectID>, Vec<GameObject<BehaviorDataContainerEnum>>) {
    if let Some(BehaviorDataContainerEnum::CameraTrackingData(camera_tracking_data)) = loop_ctx.go.behaviors_data.get(&BehaviorData::Behaviors(Behaviors::BCameraTracking)) {
        loop_ctx.camera.view_pos.x = loop_ctx.go.position.x + camera_tracking_data.x_off;
        loop_ctx.camera.view_pos.y = loop_ctx.go.position.y + camera_tracking_data.y_off;
        loop_ctx.camera.view_pos.z = loop_ctx.go.position.z + camera_tracking_data.z_off;
    }
    (vec![], vec![])
}
#[derive(Debug)]

pub struct CameraTrackingData{
    pub x_off: f32,
    pub y_off: f32,
    pub z_off: f32,
}
impl BehaviorDataContainer for CameraTrackingData{}

pub fn spawn_ball (
    loop_ctx: &mut LoopContext<BehaviorDataContainerEnum>
) -> (Vec<GameObjectID>, Vec<GameObject<BehaviorDataContainerEnum>>) {
    let mut objs_to_add = vec![];
    if loop_ctx.keys_held.contains(&Keycode::LCTRL) {
        if let Some(BehaviorDataContainerEnum::SpawnBallData(spawn_ball_data)) = loop_ctx.go.behaviors_data.get_mut(&BehaviorData::Behaviors(Behaviors::BSpawnBall)) {
            if spawn_ball_data.last_use.elapsed() >= spawn_ball_data.cooldown_length {
                let posxyz = loop_ctx.go.position;
                let ball_body_handler = loop_ctx.rigid_body_set.insert(
                    RigidBodyBuilder::dynamic().translation(vector![posxyz.x, posxyz.y]).build()
                );
                loop_ctx.collider_set.insert_with_parent(
                    ColliderBuilder::ball(1.0).active_events(ActiveEvents::COLLISION_EVENTS).build(),
                    ball_body_handler,
                    loop_ctx.rigid_body_set
                );
                let new_ball_obj = make_go_rb(
                    vec::Vec3::zero(), 
                    vec::Vec3::zero(),
                    vec::Vec3::one(),
                    vec::Vec3::zero(),
                    vec::Vec3::zero(),
                    vec::Vec3::one(),
                    loop_ctx.model_map["ball"],
                    ball_body_handler
                    ).add_behavior(Behaviors::BAttractionTo)
                    .add_behavior_data(
                        Behaviors::BAttractionTo, 
                        BehaviorDataContainerEnum::AttractionToData(AttractionToData{
                            target: loop_ctx.go.id,
                            force: 1.0
                        })
                    );
                objs_to_add.push(new_ball_obj);
                spawn_ball_data.last_use = Instant::now();
            }
        }
    }
    
    (vec![], objs_to_add)
}

#[derive(Debug)]
pub struct SpawnBallData{
    pub last_use: Instant,
    pub cooldown_length: Duration,
}
impl BehaviorDataContainer for SpawnBallData{}

pub fn attraction_to (
    loop_ctx: &mut LoopContext<BehaviorDataContainerEnum>
) -> (Vec<GameObjectID>, Vec<GameObject<BehaviorDataContainerEnum>>) {
    if let Some(BehaviorDataContainerEnum::AttractionToData(attraction_to_data)) = loop_ctx.go.behaviors_data.get_mut(&BehaviorData::Behaviors(Behaviors::BAttractionTo)) {
        if let Some(rb_handle) = loop_ctx.go.rigid_body_handle {
            if let Some(tar_rb_handle) = loop_ctx.game_obj_store.0.get(&attraction_to_data.target).unwrap().borrow().rigid_body_handle {
                let a = loop_ctx.rigid_body_set[rb_handle].translation().clone();
                let b = loop_ctx.rigid_body_set[tar_rb_handle].translation().clone();
                let c = (b - a).normalize() * attraction_to_data.force;
                loop_ctx.rigid_body_set[rb_handle].apply_impulse(c, true);
            }
        }
    }
    
    (vec![], vec![])
}

#[derive(Debug)]
pub struct AttractionToData{
    pub target: GameObjectID,
    pub force: f32
}
impl BehaviorDataContainer for AttractionToData{}

#[derive(Debug)]
pub enum BehaviorDataContainerEnum {
    ArrowControlData(ArrowControlData),
    CameraTrackingData(CameraTrackingData),
    SpawnBallData(SpawnBallData),
    AttractionToData(AttractionToData),
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum CollisionBehaviors {
    CHandleFloorCollision,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]

pub enum CollisionData {
    CollisionBehaviors(CollisionBehaviors),
}

pub trait CollisionDataContainer{}

pub fn apply_collision_behaviors<T>(
    loop_ctx: &mut LoopContext<T>,
    other: &RefCell<GameObject<T>>
) -> (Vec<GameObjectID>, Vec<GameObject<T>>) {
    let mut collision_behaviors_copy = vec![];
    for b in loop_ctx.go.collision_behaviors.iter() {
        collision_behaviors_copy.push(*b);
    }
    let mut objs_to_remove = vec![];
    let mut objs_to_add = vec![];
    for c in collision_behaviors_copy {
        match c {
            CollisionBehaviors::CHandleFloorCollision => {
                let (mut to_remove, mut to_add) = handle_floor_behaviors(loop_ctx, other);
                objs_to_remove.append(&mut to_remove);
                objs_to_add.append(&mut to_add);
            },
        }
    }
    (objs_to_remove, objs_to_add)
}

pub fn handle_floor_behaviors<T>(
    loop_ctx: &mut LoopContext<T>, 
    other: &RefCell<GameObject<T>>, 
) -> (Vec<GameObjectID>, Vec<GameObject<T>>) {
    let other_go = other.borrow_mut();
    if let Some(other_rb_handle) = other_go.rigid_body_handle {
        if loop_ctx.floor_set.contains(&other_rb_handle) && (loop_ctx.go.position.y - 0.2) >= loop_ctx.rigid_body_set[other_rb_handle].translation().y {
            loop_ctx.go.grounded = true;
        }
    }
    (vec![], vec![])
}