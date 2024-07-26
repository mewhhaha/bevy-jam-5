// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::f32::consts::PI;

use bevy::asset::AssetMetaCheck;
use bevy::color::palettes::css::{BLACK, GRAY};
use bevy::color::palettes::tailwind::GREEN_600;
use bevy::math::bounding::{Aabb2d, Bounded2d, IntersectsVolume};
use bevy::math::{vec2, VectorSpace};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use input::{Action, ActionInput, InputMappingBundle};

mod input;

const LAYER_ACTIVE: usize = 1;
const LAYER_INACTIVE: usize = 0;

const TINT_ACTIVE: Color = Color::WHITE;
const TINT_INACTIVE: Color = Color::Srgba(GRAY);

#[derive(Component, Clone)]
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
    render_layers: RenderLayers,
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
            render_layers: RenderLayers::layer(LAYER_INACTIVE),
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

fn system_setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                order: LAYER_INACTIVE as isize,
                clear_color: ClearColorConfig::Custom(Color::Srgba(BLACK)),
                ..default()
            },

            ..default()
        },
        RenderLayers::layer(LAYER_INACTIVE),
    ));
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                order: LAYER_ACTIVE as isize,
                clear_color: ClearColorConfig::Custom(Color::Srgba(BLACK)),
                ..default()
            },
            ..default()
        },
        RenderLayers::layer(LAYER_ACTIVE),
    ));
}

fn system_setup_entities(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    let hand_open_image = asset_server.load::<Image>("hand-open.png");
    let cycle_image = asset_server.load::<Image>("cycle.png");

    let hand_bundle = (
        Hand,
        Progress(0.5),
        Speed(-0.5),
        Collision::Rectangle(Rectangle::new(128., 32.)),
        SpriteBundle {
            texture: hand_open_image,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(64.0)),
                color: Color::Srgba(GRAY),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0., 0., 2.)),
            ..default()
        },
        RenderLayers::layer(LAYER_INACTIVE),
    );

    commands
        .spawn(CycleBundle::new(&cycle_image).translation(vec2(0., 0.)))
        .with_children(|parent| {
            parent.spawn((Active, hand_bundle.clone()));
        });

    commands
        .spawn(CycleBundle::new(&cycle_image).translation(vec2(192., 0.)))
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
        RenderLayers::layer(LAYER_INACTIVE),
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

fn debug_show_collision_gizmos(
    mut show: Local<bool>,
    action_input: Res<ActionInput>,
    mut gizmos: Gizmos,
    query: Query<(&GlobalTransform, &Collision)>,
) {
    if action_input.just_pressed(Action::DebugShowCollisions) {
        *show = !*show;
    }

    if !*show {
        return;
    }
    for (transform, collision) in query.iter() {
        let translation = transform.translation().xy();
        match collision {
            Collision::Rectangle(rect) => gizmos.primitive_2d(rect, translation, 0., GREEN_600),
        }
    }
}

#[derive(Resource, Default)]
struct Overlap {
    overlaps: Vec<(Entity, Entity)>,
}

impl Overlap {
    fn update(&mut self, overlaps: Vec<(Entity, Entity)>) {
        self.overlaps = overlaps;
    }

    fn with(&self, entity: Entity) -> Vec<Entity> {
        self.overlaps
            .iter()
            .filter_map(|(e1, e2)| if *e1 == entity { Some(*e2) } else { None })
            .collect()
    }
}

fn rectangle_aabb(rect: &Rectangle, transform: &GlobalTransform) -> Aabb2d {
    let (_, rotation, translation) = transform.to_scale_rotation_translation();

    rect.aabb_2d(translation.truncate(), rotation.to_euler(EulerRot::YXZ).2)
}

fn system_check_overlap(
    query: Query<(Entity, &GlobalTransform, &Collision)>,
    mut current_overlap: ResMut<Overlap>,
) {
    let mut overlaps = vec![];

    for [(e1, t1, c1), (e2, t2, c2)] in query.iter_combinations() {
        match (c1, c2) {
            (Collision::Rectangle(r1), Collision::Rectangle(r2)) => {
                let aab1 = rectangle_aabb(r1, t1);
                let aab2 = rectangle_aabb(r2, t2);
                if aab1.intersects(&aab2) {
                    overlaps.push((e1, e2));
                    overlaps.push((e2, e1));
                }
            }
        }
    }

    current_overlap.update(overlaps)
}

fn system_grab_toggle(
    mut commands: Commands,
    overlap: Res<Overlap>,
    active: Query<(Entity, &Speed, Option<&Holding>), (With<Hand>, With<Active>)>,
    hand_overs: Query<(Entity, &Speed), (With<Hand>, Without<Active>)>,
    mut items: Query<(Entity, &mut Transform), With<Item>>,
    action_input: Res<ActionInput>,
) {
    if !action_input.just_pressed(Action::Grab) {
        return;
    }

    let Ok((entity, Speed(speed), maybe_holding)) = active.get_single() else {
        return;
    };

    match maybe_holding {
        Some(Holding(Some(item))) => {
            let overlaps = overlap.with(*item);
            let is_overlapping = overlaps.into_iter().find_map(|e| hand_overs.get(e).ok());

            if let Some((other, Speed(speed_other))) = is_overlapping {
                commands.entity(other).insert(Holding(Some(*item)));
                commands.entity(other).insert(Active);
                commands
                    .entity(other)
                    .insert(Speed(speed_other.abs() * -speed.signum()));
                commands.entity(*item).set_parent_in_place(other);
                commands.entity(entity).remove::<Active>();
                commands.entity(entity).remove::<Holding>();
            } else {
                commands.entity(*item).remove_parent_in_place();
                commands.entity(entity).remove::<Holding>();
            }
        }
        Some(_) => {
            commands.entity(entity).remove::<Holding>();
        }
        _ => {
            if let Some(item) = overlap
                .with(entity)
                .into_iter()
                .find(|e| items.get_mut(*e).is_ok())
            {
                commands.entity(item).set_parent_in_place(entity);
                commands.entity(entity).insert(Holding(Some(item)));
            } else {
                commands.entity(entity).insert(Holding(None));
            }
        }
    }
}

fn move_towards_active_hand(query: Query<&Holding>, mut items: Query<(&mut Transform, &Item)>) {
    let Ok(Holding(Some(item))) = query.get_single() else {
        return;
    };

    let Ok((mut transform, _)) = items.get_mut(*item) else {
        return;
    };

    transform.translation = transform.translation.lerp(Vec3::ZERO, 0.1);
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

fn system_set_render_layer(
    mut query: Query<(Entity, &mut RenderLayers), (With<Hand>, With<Active>)>,
    mut others: Query<&mut RenderLayers, (Or<(With<Item>, With<Hand>)>, Without<Active>)>,
    overlap: Res<Overlap>,
) {
    for (mut render_layers) in &mut others {
        *render_layers = RenderLayers::layer(LAYER_INACTIVE);
    }

    if let Ok((entity, mut render_layers)) = query.get_single_mut() {
        *render_layers = RenderLayers::layer(LAYER_ACTIVE);

        for other in overlap.with(entity) {
            if let Ok(mut render_layers) = others.get_mut(other) {
                *render_layers = RenderLayers::layer(LAYER_ACTIVE);
            }
        }
    };
}

fn system_tint_layers(
    mut query: Query<(&mut Sprite, &RenderLayers), Or<(With<Hand>, With<Item>)>>,
) {
    for (mut sprite, render_layers) in &mut query {
        if render_layers == &RenderLayers::layer(LAYER_ACTIVE) && sprite.color != TINT_ACTIVE {
            sprite.color = TINT_ACTIVE;
        } else if render_layers == &RenderLayers::layer(LAYER_INACTIVE)
            && sprite.color != TINT_INACTIVE
        {
            sprite.color = TINT_INACTIVE;
        }
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
        .init_resource::<Overlap>()
        .observe(on_add_grab)
        .observe(on_remove_grab)
        .add_systems(Startup, system_setup_camera)
        .add_systems(Startup, system_setup_entities)
        .add_systems(PreUpdate, system_check_overlap)
        .add_systems(Update, system_cycle_hand)
        .add_systems(Update, system_progress)
        .add_systems(Update, system_grab_toggle)
        .add_systems(Update, move_towards_active_hand)
        .add_systems(Update, system_set_render_layer)
        .add_systems(Update, system_tint_layers)
        .add_systems(PostUpdate, debug_show_collision_gizmos)
        .add_systems(PostUpdate, system_grid_gizmo)
        .run();
}
