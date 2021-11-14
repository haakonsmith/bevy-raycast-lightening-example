use bevy::prelude::*;
use bevy::reflect::List;
use bevy_prototype_debug_lines::*;
use c2::{self, RayCast};
use c2::{prelude::*, Capsule, Circle, Poly, Ray, Rotation, Transformation};
use std::cmp::min;
use std::f32::consts::PI;
use bevy_mod_raycast::{DefaultPluginState, DefaultRaycastingPlugin, RayCastMesh, RayCastSource};


const PLAYER_SPRITE: &str = "img_test.png";

pub struct Materials {
    player_material: Handle<ColorMaterial>,
}

// components
pub struct Player;

pub struct MyRaycastSet;

pub struct ShadowCaster;

fn C2into(vec: c2::Vec2) -> Vec3 {
    return Vec3::new(vec.x(), vec.y(), 0.0);
}

// main

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin)
        .add_plugin(DefaultRaycastingPlugin::<MyRaycastSet>::default())
        .add_startup_system(setup.system())
        .add_startup_stage(
            "setup_game_actors",
            SystemStage::single(spawn_player.system()),
        )
        .add_system(draw_occulsion_debug_bounds.system())
        .add_system(cast_rays.system())
        .add_system(player_movement.system())
        .add_system(player_mouse.system())
        .run();
}

// systems

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.insert_resource(Materials {
        player_material: materials.add(asset_server.load(PLAYER_SPRITE).into()),
    });

    commands
        .spawn()
        .insert(RayCastSource::<MyRaycastSet>::new_transform_empty());

    commands
        .spawn()
        .insert(Transform {
            translation: Vec3::new(20.5, 0.5, 1.),
            scale: Vec3::new(1., 1., 1.),
            rotation: Quat::from_rotation_z(1.),
            ..Default::default()
        })
        .insert(Poly::from_slice(&[
            [-5.0, -15.0],
            [5.0, -15.0],
            [5.0, 0.0],
            [0.0, 5.0],
            [-5.0, 0.0],
        ]))
        .insert(RayCastMesh::<MyRaycastSet>::default());
        .insert(ShadowCaster);
}

fn spawn_player(mut commands: Commands, materials: Res<Materials>) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.player_material.clone(),
            transform: Transform {
                translation: Vec3::new(0.5, 0.5, 1.),
                scale: Vec3::new(1., 1., 1.) * 0.1,
                rotation: Quat::from_rotation_z(1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Poly::from_slice(&[
            [-10.0, -10.0],
            [10.0, -10.0],
            [-10.0, 10.0],
            [10.0, 10.0],
        ]))
        .insert(RayCastMesh::<MyRaycastSet>::default());
        .insert(ShadowCaster)
        .insert(Player);
}

fn player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, With<Player>)>,
) {
    if let Ok((mut transform, _)) = query.single_mut() {
        // let dir = if keyboard_input.pressed(KeyCode::A)

        let mut dir: Vec2 = Vec2::new(0., 0.);

        if keyboard_input.pressed(KeyCode::A) {
            dir.x -= 1.;
        };
        if keyboard_input.pressed(KeyCode::D) {
            dir.x += 1.;
        };
        if keyboard_input.pressed(KeyCode::S) {
            dir.y -= 1.;
        };
        if keyboard_input.pressed(KeyCode::W) {
            dir.y += 1.;
        };

        dir *= 1.;

        transform.translation.x += dir.x;
        transform.translation.y += dir.y;
    }
}

fn player_mouse(windows: Res<Windows>, mut query: Query<(&mut Transform, With<Player>)>) {
    let window = windows.get_primary().unwrap();

    if let Ok((mut transform, _)) = query.single_mut() {
        if let Some(position) = window.cursor_position() {
            // cursor is inside the window, position given
            let m_pos = position - Vec2::new(window.width(), window.height()) / 2.;

            let diff = transform.translation.truncate() - m_pos;
            let angle = diff.y.atan2(diff.x) + PI; // Add/sub FRAC_PI here optionally

            // if angle != transform.rotation.to_axis_angle().1 {
            //     print!("{:?}\n", angle);
            // }

            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

fn draw_occulsion_debug_bounds(
    mut query: Query<(&Transform, &Poly, With<ShadowCaster>)>,
    mut lines: ResMut<DebugLines>,
) {
    for (_transform, poly, _) in query.iter_mut() {
        // let transformation = Transformation::new(
        //     c2::Vec2::new(transform.translation.x, transform.translation.y),
        //     Rotation::radians(PI / 2.0),
        // );
        let mut transform = _transform.clone();

        transform.scale = Vec3::splat(1.);

        for i in 0..(poly.count()) {
            let next_index = if i + 1 > poly.count() { 0 } else { i + 1 };

            let untransformed1 = C2into(poly.get_vert(i));
            let untransformed2 = C2into(poly.get_vert(next_index));

            lines.line(
                transform.mul_vec3(untransformed1),
                transform.mul_vec3(untransformed2),
                0.,
            );
        }

        // print!("{:?}", transform.mul_vec3(untransformed1));
    }

    // let start = Vec3::splat(-10.0);
    // let end = Vec3::splat(10.0);
    // let duration = 0.0;
    // lines.line(start, end, duration);
}

fn cast_rays(
    mut query: Query<(&Transform, &Poly, With<ShadowCaster>)>,
    mut lines: ResMut<DebugLines>,
) {
    let points = get_points_for_raycast(&mut query);

    for point in points {
        let start = c2::Vec2::new(0., 0.);
        let ray = Ray::new(start, point - start);

        // print!("{:?}", ray.end());

        let ray_cast = ray_cast_to_query(&ray, &mut query);

        // print!("{:?}\n", ray_cast);

        if let Some(cast) = ray_cast {
            draw_ray_cast(ray.clone(), &cast, &mut lines)
        }
    }

    // let ray = Ray::new(c2::Vec2::new(0., 0.), c2::Vec2::new(smallest_distance, 0.));
}

fn c2MulrvT(a: c2::Vec2, b: c2::Vec2, origin: c2::Vec2) -> c2::Vec2 {
    let point = b - origin;

    return c2::Vec2::new(
        -a.y() * point.x() + a.x() * point.y(),
        a.x() * point.x() + a.y() * point.y(),
    ) + origin;
}

fn c2MulxvT(a: Transformation, b: c2::Vec2) -> c2::Vec2 {
    return c2MulrvT(a.rotation().into(), b - a.position(), a.position());
}

fn ray_cast_to_query(
    ray: &Ray,
    query: &mut Query<(&Transform, &Poly, With<ShadowCaster>)>,
) -> Option<RayCast> {
    let mut smallest_raycast: Option<RayCast> = None;

    for (transform, poly, _) in query.iter_mut() {
        let collision_ray = Ray::new(ray.start(), ray.end());

        // print!("{:?}", ray.end());

        let transformation = Transformation::new(
            c2::Vec2::new(transform.translation.x, transform.translation.y),
            Rotation::radians(transform.rotation.to_axis_angle().1),
        );

        let cast: RayCast;

        if let Some(ray_cast) = collision_ray.cast(transform_c2_poly(transform, *poly)) {
            cast = ray_cast;
        } else {
            cast = RayCast::new(ray.end().distance(ray.start()), ray.start());
        }

        if let Some(result) = smallest_raycast {
            if cast.time_of_impact() < result.time_of_impact() {
                print!("{:?}\n", cast.time_of_impact());
                smallest_raycast = Some(cast);
            }
        } else {
            smallest_raycast = Some(cast);
        }
    }

    return smallest_raycast;
}

fn draw_ray_cast(ray: Ray, ray_cast: &RayCast, lines: &mut ResMut<DebugLines>) {
    let start = ray.start();
    let end = ray_cast.position_of_impact(ray);

    lines.line(
        Vec3::new(start.x(), start.y(), 0.),
        Vec3::new(end.x(), end.y(), 0.),
        0.,
    );
}

fn get_points_for_raycast(
    query: &mut Query<(&Transform, &Poly, With<ShadowCaster>)>,
) -> Vec<c2::Vec2> {
    let mut points = Vec::<c2::Vec2>::new();

    for (transform, poly, _) in query.iter_mut() {
        for i in 0..poly.count() {
            points.push(transform_c2(transform, poly.get_vert(i)));
        }
    }

    points.push(c2::Vec2::new(1000., 0.));
    points.push(c2::Vec2::new(-1000., 0.));

    points
}

fn transform_c2(transform: &Transform, vec: c2::Vec2) -> c2::Vec2 {
    let untransformed = C2into(vec);
    let mut copy = transform.clone();
    copy.scale = Vec3::splat(1.);
    let transformed = copy.mul_vec3(untransformed);

    c2::Vec2::new(transformed.x, transformed.y)
}

fn transform_c2_poly(transform: &Transform, poly: Poly) -> Poly {
    let mut copy = transform.clone();
    copy.scale = Vec3::splat(1.);

    let mut points = [[0. as f32; 2]; 8];

    for i in 0..poly.count() {
        let p = transform_c2(&copy, poly.get_vert(i));
        points[i] = [p.x(), p.y()];
    }

    Poly::from_array(points.len(), points)
}

// Burnt as in like, people intrepreting me talking to them and getting close even though I explicitly stated that I didn't want to get close, and we'd agreed that we'd just go with the flow as: "let's have kids together"
