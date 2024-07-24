// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::f32::consts::PI;

use bevy::asset::AssetMetaCheck;
use bevy::color::palettes::css::{GRAY, GREEN};
use bevy::color::palettes::tailwind::{GREEN_100, GREEN_600};
use bevy::math::bounding::{Aabb2d, Bounded2d, BoundingVolume, IntersectsVolume};
use bevy::math::vec2;
use bevy::prelude::*;
use input::{Action, ActionInput, InputMappingBundle};

mod input;

#[derive(Component)]
struct Holding(Option<Entity>);

#[derive(Component, Clone)]
struct Progress(f32);

#[derive(Component)]
struct Radius(f32);

#[derive(Component, Clone)]
struct Speed(f32);

#[derive(Component)]
struct Active;

#[derive(Component)]
struct Item;

#[derive(Component, Clone)]
struct Cycle;

#[derive(Component, Clone)]
struct Hand;

#[derive(Bundle)]
struct CycleBundle {
    sprite_bundle: SpriteBundle,
    radius: Radius,
    cycle: Cycle,
}

impl CycleBundle {
    fn new(texture: &Handle<Image>) -> Self {
        Self {
            cycle: Cycle,
            sprite_bundle: SpriteBundle {
                texture: texture.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(128.0)),
                    ..default()
                },
                ..default()
            },
            radius: Radius(64.0),
        }
    }

    fn radius(mut self, radius: f32) -> Self {
        self.radius = Radius(radius);
        self
    }

    fn translation(mut self, vec: Vec2) -> Self {
        self.sprite_bundle.transform.translation = vec.extend(0.);
        self
    }
}

#[derive(Component, Clone)]
enum Collision {
    Rectangle(Rectangle),
}

fn system_setup(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let hand_open_image = asset_server.load::<Image>("hand-open.png");
    let cycle_image = asset_server.load::<Image>("cycle.png");

    let hand_bundle = (
        Hand,
        Progress(0.5),
        Speed(-0.5),
        Collision::Rectangle(Rectangle::new(32., 32.)),
        SpriteBundle {
            texture: hand_open_image,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(64.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0., 0., 2.)),
            ..default()
        },
    );

    commands
        .spawn(CycleBundle::new(&cycle_image).translation(vec2(0., 0.)))
        .with_children(|parent| {
            parent.spawn((Active, hand_bundle.clone()));
        });

    commands
        .spawn(CycleBundle::new(&cycle_image).translation(vec2(128., 0.)))
        .with_children(|parent| {
            parent.spawn(hand_bundle.clone());
        });

    let texture = asset_server.load("baton.png");

    commands.spawn((
        Item,
        Collision::Rectangle(Rectangle::new(32., 32.)),
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::new(64.0, 64.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-64., 0., 1.)),
            ..default()
        },
    ));
}

fn system_progress(mut query: Query<(&mut Progress, &Speed), With<Active>>, time: Res<Time>) {
    for (mut progress, Speed(speed)) in query.iter_mut() {
        progress.0 += time.delta_seconds() * speed;
        if progress.0 > 1. {
            progress.0 = progress.0 - progress.0.trunc();
        }
    }
}

fn system_show_collision_gizmos(
    mut show: Local<bool>,
    action_input: Res<ActionInput>,
    mut gizmos: Gizmos,
    query: Query<(&Transform, &Collision)>,
) {
    if action_input.just_pressed(Action::DebugShowCollisions) {
        *show = !*show;
    }

    if !*show {
        return;
    }
    for (transform, collision) in query.iter() {
        let translation = transform.translation.xy();
        let rotation = transform.rotation.to_euler(EulerRot::YXZ).2;
        match collision {
            Collision::Rectangle(rect) => {
                gizmos.primitive_2d(rect, translation, rotation, GREEN_600)
            }
        }
    }
}

#[derive(PartialEq, Eq)]
struct Overlap(Entity, Entity);

impl Overlap {
    fn new(e1: Entity, e2: Entity) -> Self {
        if e1 < e2 {
            Self(e1, e2)
        } else {
            Self(e2, e1)
        }
    }

    fn overlaps(&self, e1: Entity, e2: Entity) -> bool {
        let incoming = Overlap::new(e1, e2);
        self == &incoming
    }
}

#[derive(Resource, Default)]
struct CurrentOverlap {
    overlaps: Vec<Overlap>,
}

impl CurrentOverlap {
    fn update(&mut self, overlaps: Vec<Overlap>) {
        self.overlaps = overlaps;
    }
}

fn rectangle_aabb(rect: &Rectangle, transform: &Transform) -> Aabb2d {
    rect.aabb_2d(
        transform.translation.truncate(),
        transform.rotation.to_euler(EulerRot::YXZ).2,
    )
}

fn system_check_overlap(
    query: Query<(Entity, &Transform, &Collision)>,
    mut current_overlap: ResMut<CurrentOverlap>,
) {
    let mut overlaps = vec![];

    for [(e1, t1, c1), (e2, t2, c2)] in query.iter_combinations() {
        match (c1, c2) {
            (Collision::Rectangle(r1), Collision::Rectangle(r2)) => {
                let aab1 = rectangle_aabb(r1, t1);
                let aab2 = rectangle_aabb(r2, t2);
                if aab1.intersects(&aab2) {
                    overlaps.push(Overlap::new(e1, e2));
                }
            }
        }
    }

    current_overlap.update(overlaps)
}

fn system_grab_toggle(
    mut commands: Commands,
    current_overlap: Res<CurrentOverlap>,
    mut query: Query<(Entity, &Transform, Option<&Holding>), (With<Hand>, With<Active>)>,
    items: Query<(Entity, &Transform), With<Item>>,
    action_input: Res<ActionInput>,
) {
    if !action_input.just_pressed(Action::Grab) {
        return;
    }

    let Some((entity, hand, holding)) = query.iter().next() else {
        return;
    };

    if holding.is_some() {
        commands.entity(entity).remove::<Holding>();
        return;
    } else {
        let hand_aabb = Aabb2d::new(hand.translation.truncate(), Vec2::splat(32.));

        let item = items.iter().find_map(|(entity, item)| {
            let item_aab = Aabb2d::new(item.translation.truncate(), Vec2::splat(32.));
            if hand_aabb.intersects(&item_aab) {
                Some(entity)
            } else {
                None
            }
        });

        commands.entity(entity).insert(Holding(item));
    }
}

fn system_item_in_hand(
    query: Query<(&Transform, &Holding), (With<Active>, With<Hand>, Without<Item>)>,
    mut items: Query<&mut Transform, With<Item>>,
) {
    for (hand, Holding(held)) in query.iter() {
        let Some(entity) = held else {
            continue;
        };

        let Ok(mut item) = items.get_mut(*entity) else {
            continue;
        };

        item.translation = hand.translation.truncate().extend(item.translation.z);
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

fn on_add_grab(
    trigger: Trigger<OnAdd, Holding>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Handle<Image>>,
) {
    if let Ok(mut sprite) = query.get_mut(trigger.entity()) {
        *sprite = asset_server.load("hand-closed.png");
    }
}

fn system_cycle_hand(
    mut query: Query<(&Children, &Radius), With<Cycle>>,
    mut hands: Query<(&mut Transform, &Progress), With<Hand>>,
) {
    for (children, radius) in query.iter_mut() {
        for child in children.iter() {
            let Ok((mut hand, Progress(progress))) = hands.get_mut(*child) else {
                continue;
            };

            let angle = progress * 2. * PI;
            let offset = Vec2::new(angle.cos(), angle.sin()) * radius.0;
            hand.translation.x = offset.x;
            hand.translation.y = offset.y;
        }
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
        .init_resource::<CurrentOverlap>()
        .add_systems(Startup, system_setup)
        .add_systems(PreUpdate, system_check_overlap)
        .add_systems(Update, system_cycle_hand)
        .add_systems(Update, system_progress)
        .add_systems(PostUpdate, system_grid_gizmo)
        .add_systems(Update, system_grab_toggle)
        .add_systems(Update, system_item_in_hand)
        .add_systems(Update, system_show_collision_gizmos)
        .observe(on_add_grab)
        .observe(on_remove_grab)
        .run();
}
