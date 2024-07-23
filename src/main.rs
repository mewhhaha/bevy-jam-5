// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::{f32::consts::PI, time::Duration};

use bevy::math::bounding::{Aabb2d, IntersectsVolume};
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::{asset::AssetMetaCheck, transform::commands};
use input::{Action, ActionInput, InputMappingBundle};

mod input;

#[derive(Component)]
struct Holding(Option<Entity>);

#[derive(Component)]
struct Progress(f32);

#[derive(Component)]
struct Radius(f32);

#[derive(Component)]
struct Speed(f32);

#[derive(Component)]
struct Center(Vec2);

#[derive(Component)]
struct Active;

#[derive(Component)]
struct UnclenchTimer(Timer);

#[derive(Component)]
struct Item;

fn system_setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut config_store: ResMut<GizmoConfigStore>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2dBundle::default());

    let hand_open = asset_server.load::<Image>("hand-open.png");
    commands.spawn((
        Progress(0.0),
        Speed(-0.5),
        Radius(64.0),
        Active,
        Center(Vec2::ZERO),
        SpriteBundle {
            texture: hand_open.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            ..default()
        },
    ));
    commands.spawn((
        Progress(0.5),
        Speed(-0.5),
        Radius(60.0),
        Center(vec2(128., 0.)),
        SpriteBundle {
            texture: hand_open,
            sprite: Sprite {
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            ..default()
        },
    ));

    let texture = asset_server.load("fishes.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 9, 8, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.spawn((
        Item,
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-64., 0., 0.)),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout,
            index: 0,
        },
    ));

    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line_width = 6.0;
}

fn system_progress(mut query: Query<(&mut Progress, &Speed), With<Active>>, time: Res<Time>) {
    for (mut progress, Speed(speed)) in query.iter_mut() {
        progress.0 += time.delta_seconds() * speed;
        if progress.0 > 1. {
            progress.0 = progress.0 - progress.0.trunc();
        }
    }
}

fn grab_toggle(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &mut Handle<Image>), With<Active>>,
    items: Query<(Entity, &Transform), With<Item>>,
    action_input: Res<ActionInput>,
    asset_server: Res<AssetServer>,
) {
    if action_input.just_pressed(Action::Grab) {
        let hand_open = asset_server.load::<Image>("hand-open.png");
        let hand_closed = asset_server.load::<Image>("hand-closed.png");

        for (entity, hand, mut sprite) in query.iter_mut() {
            if sprite.id() != hand_open.id() {
                *sprite = hand_open.clone();
                commands.entity(entity).remove::<Holding>();
            } else {
                *sprite = hand_closed.clone();

                let hand_aabb = Aabb2d::new(hand.translation.truncate(), Vec2::splat(32.));

                let item = items.iter().find_map(|(entity, item)| {
                    let item_aab = Aabb2d::new(item.translation.truncate(), Vec2::splat(32.));
                    if hand_aabb.intersects(&item_aab) {
                        println!("grabbed item");
                        Some(entity)
                    } else {
                        println!("missed item");
                        None
                    }
                });

                commands.entity(entity).insert(Holding(item));
            }
        }
    }
}

fn move_with_hand(
    query: Query<(&Transform, &Holding), (With<Active>, Without<Item>)>,
    mut items: Query<&mut Transform, With<Item>>,
) {
    for (hand, Holding(held)) in query.iter() {
        let Some(entity) = held else {
            continue;
        };

        let Ok(mut item) = items.get_mut(*entity) else {
            continue;
        };

        item.translation = hand.translation.clone();
    }
}

fn on_remove_grab(
    trigger: Trigger<OnRemove, Holding>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Handle<Image>>,
) {
    if let Ok(mut sprite) = query.get_mut(trigger.entity()) {
        *sprite = asset_server.load("hand-open.png");
    }
}

fn on_insert_grab(
    trigger: Trigger<OnInsert, Holding>,
    query: Query<&Holding>,
    mut commands: Commands,
) {
    let mut entity = commands.entity(trigger.entity());
    match query.get(trigger.entity()) {
        Ok(Holding(Some(_))) => {
            entity.remove::<UnclenchTimer>();
        }
        Ok(Holding(None)) => {
            entity.insert(UnclenchTimer(Timer::from_seconds(0.3, TimerMode::Once)));
        }
        _ => {}
    }
    if let None = query.get(trigger.entity()).ok() {}
}

fn on_add_grab(
    trigger: Trigger<OnAdd, Holding>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Handle<Image>>,
) {
    if let Ok(mut sprite) = query.get_mut(trigger.entity()) {
        *sprite = asset_server.load("hand-closed.png");
    }
}

fn unclench_hand(
    time: Res<Time>,
    mut query: Query<(Entity, &mut UnclenchTimer)>,
    mut commands: Commands,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.finished() {
            commands.entity(entity).remove::<(Holding, UnclenchTimer)>();
        }
    }
}

fn system_cycle_gizmo(
    mut gizmos: Gizmos,
    mut query: Query<(&Center, &Progress, &Speed, &Radius, &mut Transform)>,
) {
    for (Center(center), Progress(progress), Speed(speed), Radius(radius), mut transform) in
        query.iter_mut()
    {
        transform.translation = Vec3::new(
            center.x + radius * (progress * PI * 2.).cos(),
            center.y + radius * (progress * PI * 2.).sin(),
            0.0,
        );

        gizmos.circle_2d(*center, *radius, Color::WHITE);
    }
}

fn system_grid_gizmo(mut gizmos: Gizmos) {
    gizmos.grid_2d(
        Vec2::ZERO,
        0.,
        UVec2 { x: 100, y: 100 },
        vec2(64., 64.),
        Color::linear_rgb(0.2, 0.2, 0.2),
    );
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Wasm builds will check for meta files (that don't exist) if this isn't set.
            // This causes errors and even panics in web builds on itch.
            // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
            meta_check: AssetMetaCheck::Never,
            ..default()
        }))
        .add_plugins(InputMappingBundle)
        .add_systems(Startup, system_setup)
        .add_systems(Update, system_cycle_gizmo)
        .add_systems(Update, system_progress)
        .add_systems(Update, system_grid_gizmo)
        .add_systems(Update, grab_toggle)
        .add_systems(Update, unclench_hand)
        .add_systems(Update, move_with_hand)
        .observe(on_add_grab)
        .observe(on_remove_grab)
        .observe(on_insert_grab)
        .run();
}
