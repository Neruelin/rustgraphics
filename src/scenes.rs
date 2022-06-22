#![allow(unused_variables)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use rapier2d::prelude::*;
use ultraviolet::vec;
use crate::gllib::*;
use crate::behaviors::*;

pub fn make_scene_empty<T>(
    ctx: &mut Context<T>,
    model_map: &HashMap<&str, usize>,
) -> Box<dyn Fn(&ShaderProgram, &DrawableObject)> {
    ctx.camera.view_pos = vec::Vec3::new(-20.0, 20.0, 20.0);
    ctx.camera.view_rot = vec::Vec3::new(0.0, -20.0, -45.0);
    ctx.camera.light_position = vec::Vec3::new(0.0, 50.0, 0.0);

    Box::new(move |shader: &ShaderProgram, draw: &DrawableObject| {})
}

pub fn make_scene_waves<T>(
    ctx: &mut Context<T>,
    model_map: &HashMap<&str, usize>,
) -> Box<dyn Fn(&ShaderProgram, &DrawableObject)> {
    ctx.camera.view_pos = vec::Vec3::new(-20.0, 10.0, 20.0);
    ctx.camera.view_rot = vec::Vec3::new(0.0, -20.0, -45.0);
    ctx.camera.light_position = vec::Vec3::new(0.0, 50.0, 0.0);

    let key = ctx.game_obj_store.add(make_go(
        vec::Vec3::new( 0.0, 0.0, 0.0), 
        vec::Vec3::zero(),
        vec::Vec3::one(),
        vec::Vec3::zero(),
        vec::Vec3::zero(),
        vec::Vec3::one(),
        2
    ));
    // obj_store.0.get_mut(&key).unwrap().borrow_mut().behaviors.push(Box::new(ArrowControl()));
    
    for x in -10..10 {
        for y in -10..10 {
            let x_off = if ((x % 2) == 0) != ((y % 2) == 0) {1} else {0};
            ctx.game_obj_store.add(make_go(
                vec::Vec3::new( 
                ((2*x)) as f32, 
                -2.0 - x_off as f32, 
                (2*y) as f32), 
                vec::Vec3::zero(),
                vec::Vec3::one(),
                vec::Vec3::zero(),
                vec::Vec3::zero(),
                vec::Vec3::one(),
                1
            ));
        }
    }

    Box::new(move |shader: &ShaderProgram, draw: &DrawableObject| {
        shader.set_3_float(UNI_ID[UniEnum::DiffuseColor as usize], draw.position.x / 10.0 + 1.0, draw.position.y + 3.0, draw.position.z / 10.0 + 1.0);
    })
}

pub fn make_scene_physics(
    ctx: &mut Context<BehaviorDataContainerEnum>,
    model_map: &HashMap<&str, usize>,
) -> Box<dyn Fn(&ShaderProgram, &DrawableObject)> {
    ctx.camera.view_pos = vec::Vec3::new(0.0, 1.0, 5.0);
    ctx.camera.view_rot = vec::Vec3::new(0.0, 0.0, -90.0);
    ctx.camera.light_position = vec::Vec3::new(100.0, 100.0, 0.0);

    clear_color(0.5, 0.5, 1.0, 1.0);

    /* floor collider */
    let position = vec::Vec3::new(0.0,0.0,0.0);
    let floor_body_handle = ctx.rigid_body_set.insert(
        RigidBodyBuilder::kinematic_position_based().translation(vector![position.x, position.y]).build()
    );
    ctx.collider_set.insert_with_parent(ColliderBuilder::cuboid(100.0, 1.0).build(), floor_body_handle, &mut ctx.rigid_body_set);
    let id0 = ctx.game_obj_store.add(make_go_rb(
        position, 
        vec::Vec3::zero(),
        vec::Vec3::new(1.0, 1.0, 1.0),
        vec::Vec3::new(0.0, 0.0, 0.0),
        vec::Vec3::zero(),
        vec::Vec3::new(100.0, 1.0, 100.0),
        1,
        floor_body_handle
    )
    );
    ctx.floor_set.insert(floor_body_handle);

    /* player */
    let position = vec::Vec3::new(0.0, 5.0,0.0);
    let ball_body_handle2 = ctx.rigid_body_set.insert(
        RigidBodyBuilder::dynamic().translation(vector![position.x, position.y]).lock_rotations().build()
    );
    ctx.collider_set.insert_with_parent(
        ColliderBuilder::ball(1.0).friction(0.0).active_events(ActiveEvents::COLLISION_EVENTS).build(), 
        ball_body_handle2, 
        &mut ctx.rigid_body_set
    );
    let id2 = ctx.game_obj_store.add(make_go_rb::<BehaviorDataContainerEnum>(
        vec::Vec3::zero(), 
        vec::Vec3::zero(),
        vec::Vec3::one(),
        vec::Vec3::zero(),
        vec::Vec3::zero(),
        vec::Vec3::one(),
        model_map["cone_ring"],
        ball_body_handle2
        )
        .add_behavior(Behaviors::BArrowControl)
        .add_behavior_data(
            Behaviors::BArrowControl, 
            BehaviorDataContainerEnum::ArrowControlData(ArrowControlData{accel: 10.0, max_speed: 5.0})
        )
        .add_behavior(Behaviors::BCameraTracking)
        .add_behavior_data(
            Behaviors::BCameraTracking, 
            BehaviorDataContainerEnum::CameraTrackingData(CameraTrackingData{x_off: 0.0, y_off: 2.0, z_off: 20.0})
        )
        .add_behavior(Behaviors::BSpawnBall)
        .add_behavior_data(
            Behaviors::BSpawnBall, 
            BehaviorDataContainerEnum::SpawnBallData(SpawnBallData{
                cooldown_length: Duration::from_secs(2), 
                last_use: Instant::now() - Duration::from_secs(2)
            })
        )
        .add_collision_behavior(CollisionBehaviors::CHandleFloorCollision)
    );

    /* obstacles */
    for i in 0..10 {
        let position = vec::Vec3::new(5.0, (i + 1) as f32 * 2.0,0.0);
        let cube_body_handle = ctx.rigid_body_set.insert(
            RigidBodyBuilder::dynamic().translation(vector![position.x, position.y]).build()
        );
        ctx.collider_set.insert_with_parent(
            ColliderBuilder::cuboid(1.0, 1.0).active_events(ActiveEvents::COLLISION_EVENTS).build(), 
            cube_body_handle, 
            &mut ctx.rigid_body_set
        );
        let id2 = ctx.game_obj_store.add(make_go_rb(
            vec::Vec3::zero(), 
            vec::Vec3::zero(),
            vec::Vec3::one(),
            vec::Vec3::zero(),
            vec::Vec3::zero(),
            vec::Vec3::one(),
            model_map["cube"],
            cube_body_handle
            ).add_behavior(Behaviors::BAttractionTo)
            .add_behavior_data(
                Behaviors::BAttractionTo, 
                BehaviorDataContainerEnum::AttractionToData(AttractionToData{
                    target: id2,
                    force: 0.25
                })
            )
        );
        ctx.floor_set.insert(cube_body_handle);
    }   

    /* background */
    for z in 0..4 {
        for i in -50..50 {
            if ((i * 2) + (2 * z)) % (z + 1) != 0 {continue;}
            ctx.game_obj_store.add(
                make_go(
                    vec::Vec3::new(((i * 2) + (2 * z)) as f32, 1.0, (z + 1) as f32 * -2.0), 
                    vec::Vec3::zero(), 
                    vec::Vec3::one(), 
                    vec::Vec3::zero(), 
                    vec::Vec3::zero(), 
                    vec::Vec3::one() * (z*2 + 1) as f32, 
                    0
                )
            );
        }
    }
    
    Box::new(move |shader: &ShaderProgram, draw: &DrawableObject| {
        // shader.set_3_float(UNI_ID[UniEnum::DiffuseColor as usize], draw.position.x / 10.0 + 1.0, draw.position.y + 3.0, draw.position.z / 10.0 + 1.0);
    })
}