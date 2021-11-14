use bevy::reflect::List;
use bevy::render::pipeline::PrimitiveTopology;
use bevy::{prelude::*, window};
use bevy_prototype_debug_lines::*;
// use parry2d::math::{Point, Vector};
use parry2d::na::{ComplexField, Isometry2, Norm, Point2, Vector2};
use parry2d::query::{Ray, RayCast};
use parry2d::shape::ConvexPolygon;
use std::f32::consts::PI;
use std::convert::TryFrom;

const PLAYER_SPRITE: &str = "img_test.png";

pub struct Materials {
    player_material: Handle<ColorMaterial>,
}

// components
pub struct Player;

pub struct MyRaycastSet;

pub struct ShadowCaster;

pub struct ShadowMeshData {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}

// main

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin)
        .add_startup_system(setup.system())
        .add_startup_stage(
            "setup_game_actors",
            SystemStage::single(spawn_player.system()),
        )
        .add_system(player_movement.system())
        .add_system(player_mouse.system())
        .add_system(draw_occulsion_debug_bounds.system())
        .add_system(cast_rays.system())
        .run();
}

// systems

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
) {
    let window = windows.get_primary().unwrap();

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.insert_resource(Materials {
        player_material: materials.add(asset_server.load(PLAYER_SPRITE).into()),
    });

    commands
        .spawn()
        .insert(Transform {
            translation: Vec3::new(0., 0., 1.),
            scale: Vec3::new(1., 1., 1.),
            // rotation: Quat::from_rotation_z(1.),
            ..Default::default()
        })
        .insert(
            parry2d::shape::ConvexPolygon::from_convex_polyline(vec![
                Point2::new(-window.width() / 2., -window.height() / 2. + 10.),
                Point2::new(-window.width() / 2., -window.height() / 2. - 10.),
                Point2::new(window.width() / 2., -window.height() / 2. - 10.),
                Point2::new(window.width() / 2., -window.height() / 2. + 10.),
            ])
            .unwrap(),
        )
        .insert(ShadowCaster);

    commands
        .spawn()
        .insert(Transform {
            translation: Vec3::new(20.5, 0.5, 1.),
            scale: Vec3::new(1., 1., 1.),
            rotation: Quat::from_rotation_z(1.),
            ..Default::default()
        })
        .insert(
            parry2d::shape::ConvexPolygon::from_convex_polyline(vec![
                Point2::new(-5.0, -15.0),
                Point2::new(5.0, -15.0),
                Point2::new(5.0, 15.0),
            ])
            .unwrap(),
        )
        .insert(ShadowCaster);
}

fn spawn_player(mut commands: Commands, materials: Res<Materials>) {
    let poly = parry2d::shape::ConvexPolygon::from_convex_polyline(vec![
        Point2::new(-15.0, -15.0),
        Point2::new(15.0, -15.0),
        Point2::new(15.0, 15.0),
        Point2::new(-15.0, 15.0),
    ])
    .unwrap();

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
        .insert(poly)
        .insert(ShadowCaster)
        .insert(Player);
}

fn player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, With<Player>)>,
) {
    if let Ok((mut transform, _)) = query.single_mut() {
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

        dir *= 5.;

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

            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

fn draw_occulsion_debug_bounds(
    mut query: Query<(&Transform, &parry2d::shape::ConvexPolygon)>,
    mut lines: ResMut<DebugLines>,
) {
    for (transform, poly) in query.iter_mut() {
        let points = poly.points();

        let iso = to_parry(transform);

        for i in 0..points.len() {
            let next_index = if i + 1 > points.len() - 1 { 0 } else { i + 1 };

            let p1 = iso * points[i];
            let p2 = iso * points[next_index];

            lines.line(Vec3::new(p1.x, p1.y, 0.), Vec3::new(p2.x, p2.y, 0.), 0.);
        }
    }
}

fn cast_rays(
    mut query: Query<(
        &Transform,
        &parry2d::shape::ConvexPolygon,
        With<ShadowCaster>,
    )>,
    mut lines: ResMut<DebugLines>,
) {
    // let mut shadow_mesh = ShadowMeshData {
    //     indices: vec![],
    //     vertices: vec![],
    // };

    let points = get_points_for_raycast(&mut query);

    let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(points.len() * 3);
    let origin = Point2::<f32>::new(0., 0.);

    for point in points {

        let diff = point - origin;
        let angle = diff.y.atan2(diff.x);

        let min = Ray::new(
            origin,
            Vector2::new((angle - 0.001).cos(), (angle - 0.001).sin()),
        );
        let max = Ray::new(
            origin,
            Vector2::new((angle + 0.001).cos(), (angle + 0.001).sin()),
        );
        let ray = Ray::new(origin, (point - origin).normalize());

        let ray_cast = ray_cast_to_query(&ray, 1000., &mut query);

        if let Some(cast) = ray_cast {
            
            draw_ray_cast(ray, cast, &mut lines);
        } else {
            draw_ray_cast(ray, (point - origin).magnitude(), &mut lines);
        }

        let ray_cast = ray_cast_to_query(&min, 10000., &mut query);

        if let Some(cast) = ray_cast {
            let end = ray.dir * cast;
            vertices.push([end.x, end.y, 0.]);
            draw_ray_cast(min, cast, &mut lines);
        } else {
            draw_ray_cast(min, 1000., &mut lines);
        }

        let ray_cast = ray_cast_to_query(&max, 10000., &mut query);

        if let Some(cast) = ray_cast {
            let end = ray.dir * cast;
            vertices.push([end.x, end.y, 0.]);
            draw_ray_cast(max, cast, &mut lines);
        } else {
            draw_ray_cast(max, 1000., &mut lines);
        }

        // let mut indices : Vec::<u32> = Vec::new();
        // let mut colors  : Vec::<[f32; 3]> = Vec::new();

        // for vertex in &shadow_mesh.vertices {
        //     vertices.push([vertex.x, vertex.y, vertex.z]);
        // }

        // let mesh = Mesh::new(PrimitiveTopology::LineList);

        vertices.sort_by(|a,b| {
            // (a[0]*a[0] + a[1]*a[1]).partial_cmp(&(b[0]*b[0] + b[1]*b[1])).unwrap()
            // let distance_a = origin.to_homogeneous().metric_distance(&parry2d::na::Vector3::new(a[0], a[1], a[2]));
            // let distance_b = origin.to_homogeneous().metric_distance(&parry2d::na::Vector3::new(b[0], b[1], b[2]));
            let a_diff = Vec2::from_slice_unaligned(a) - Vec2::new(origin.x, origin.y);
            let b_diff = Vec2::from_slice_unaligned(b) - Vec2::new(origin.x, origin.y);

            a_diff.y.atan2(a_diff.x).partial_cmp(&b_diff.y.atan2(b_diff.x)).unwrap()

            // (distance_a).partial_cmp(&(distance_b)).unwrap()
        });

        // print!("{:?}\n", &vertices);

        draw_mesh_outline_debug(&vertices, &mut lines);

        // let ray_cast = ray_cast_to_query(&ray, 1000., &mut query, Some(poly));
    }
    // let ray = Ray::new(c2::Vec2::new(0., 0.), c2::Vec2::new(smallest_distance, 0.));
}

fn draw_mesh_outline_debug(vertices: &Vec<[f32; 3]>, lines: &mut ResMut<DebugLines>) {
    for i in 0..vertices.len() {
        let next_index = if i + 1 > vertices.len() - 1 { 0 } else { i + 1 };

        lines.line(
            Vec3::from_slice_unaligned(&vertices[i]),
            Vec3::from_slice_unaligned(&vertices[next_index]),
            0.
        )
    }
}

/// Returns a time of impact
fn ray_cast_to_query(
    ray: &Ray,
    max_toi: f32,
    query: &mut Query<(&Transform, &ConvexPolygon, With<ShadowCaster>)>,
) -> Option<f32> {
    let mut smallest_raycast: Option<f32> = None;

    for (transform, poly, _) in query.iter_mut() {
        let transformation = to_parry(transform);

        if let Some(ray_cast) = poly.cast_ray(&transformation, ray, max_toi, true) {
            if let Some(smallest_cast) = smallest_raycast {
                if smallest_cast > ray_cast {
                    smallest_raycast = Some(ray_cast);
                }
            } else {
                smallest_raycast = Some(ray_cast);
            }
        }
    }

    return smallest_raycast;
}

fn draw_ray_cast(ray: parry2d::query::Ray, toi: f32, lines: &mut ResMut<DebugLines>) {
    return;

    let start = ray.origin;
    let end = ray.dir * toi;
    let my_span = info_span!("draw_ray_cast()", name = "draw_ray_cast()");
    {
        let guard = my_span.enter();

        lines.line(
            Vec3::new(start.x, start.y, 0.),
            Vec3::new(end.x, end.y, 0.),
            0.,
        );
    }
}

fn get_points_for_raycast(
    query: &mut Query<(&Transform, &ConvexPolygon, With<ShadowCaster>)>,
) -> Vec<Point2<f32>> {
    let mut points = Vec::<Point2<f32>>::new();

    for (transform, poly, _) in query.iter_mut() {
        let iso = to_parry(transform);

        let _points = poly.points();

        for i in 0.._points.len() {
            points.push(iso * _points[i]);
        }
    }

    points
}

fn to_parry(transform: &Transform) -> parry2d::math::Isometry<f32> {
    return Isometry2::new(
        parry2d::na::Vector2::new(transform.translation.x, transform.translation.y),
        transform.rotation.to_axis_angle().1,
    );
}

fn render_shadow_mesh() {}

// Burnt as in like, people intrepreting me talking to them and getting close even though I explicitly stated that I didn't want to get close, and we'd agreed that we'd just go with the flow as: "let's have kids together"
