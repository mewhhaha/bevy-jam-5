// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::f32::consts::PI;
use std::time::Duration;

use bevy::color::palettes::css::GRAY;
use bevy::color::palettes::tailwind::{
    BLUE_100, GREEN_100, ORANGE_100, PINK_100, PURPLE_100, RED_100, TEAL_100, YELLOW_100,
};
use bevy::math::bounding::{BoundingCircle, IntersectsVolume};
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use input::{Action, ActionInput};

use crate::input;

const LAYER_ACTIVE: usize = 1;
const LAYER_INACTIVE: usize = 0;
const TINT_ACTIVE: Color = Color::WHITE;
const TINT_INACTIVE: Color = Color::Srgba(GRAY);
const SPACING_CYCLE: f32 = 64.;
const RADIUS_CYCLE: f32 = 192.;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum Game {
    Playing,
    Finished,
}

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

#[derive(Resource, Default)]
struct CameraFocus(Vec2);

#[derive(Component)]
struct AfterImage;

#[derive(Component)]
struct FadeOutSpeed(f32);

#[derive(Event)]
enum GameEvent {
    Drop,
    Grab,
    GrabEmpty,
    HandOver,
}

#[derive(Component, Clone)]
pub enum Collision {
    Circle(Circle),
}

#[derive(Component, Clone)]
struct Finish;

#[derive(Component)]
struct CanHold;

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
                    custom_size: Some(Vec2::splat(RADIUS_CYCLE * 2.)),

                    ..default()
                },

                ..default()
            },
            render_layers: RenderLayers::layer(LAYER_INACTIVE),
            radius: Radius(RADIUS_CYCLE),
        }
    }

    fn translation(mut self, vec: Vec2) -> Self {
        self.sprite_bundle.transform.translation = vec.extend(0.);
        self
    }
}

#[derive(Bundle)]
struct HandBundle {
    hand: Hand,
    progress: Progress,
    collision: Collision,
    sprite: SpriteBundle,
    render_layers: RenderLayers,
    can_hold: CanHold,
}

impl HandBundle {
    fn new(texture: &Handle<Image>) -> Self {
        Self {
            hand: Hand,
            progress: Progress(0.5),
            collision: Collision::Circle(Circle::new(64.)),
            sprite: SpriteBundle {
                texture: texture.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(64.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(0., 0., 2.)),
                ..default()
            },
            can_hold: CanHold,
            render_layers: RenderLayers::layer(LAYER_INACTIVE),
        }
    }
}

fn system_setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                order: LAYER_INACTIVE as isize,
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
    let finish_image = asset_server.load::<Image>("finish.png");
    let baton_image = asset_server.load("statue.png");

    enum Place {
        Cycle(Vec2, f32),
        CycleStart(Vec2, f32),
        Baton(Vec2),
        Finish(Vec2),
    }

    let cycles = [
        Place::Baton(vec2(-0.5, 0.)),
        Place::CycleStart(vec2(0., 0.), 0.5),
        Place::Cycle(vec2(1., 0.), 1.),
        Place::Cycle(vec2(2., 0.), 1.5),
        Place::Cycle(vec2(3., 0.), 2.),
        // Place::Cycle(vec2(4., 0.), 2.5),
        // Place::Cycle(vec2(5., 0.), 3.),
        // Place::Cycle(vec2(6., 0.), 3.5),
        // Place::Cycle(vec2(7., 0.), 4.),
        // Place::Cycle(vec2(8., 0.), 4.5),
        // Place::Cycle(vec2(9., 0.), 5.),
        Place::Finish(vec2(3.5, 0.)),
    ];

    for place in &cycles {
        let conversion = RADIUS_CYCLE * 2. + SPACING_CYCLE;
        match place {
            Place::Cycle(position, speed) | Place::CycleStart(position, speed) => {
                commands
                    .spawn(CycleBundle::new(&cycle_image).translation(*position * conversion))
                    .with_children(|parent| {
                        let mut hand = parent.spawn(HandBundle::new(&hand_open_image));
                        hand.insert(Speed(*speed));
                        if let Place::CycleStart(_, _) = place {
                            hand.insert(Active);
                        }
                    });
            }
            Place::Finish(position) => {
                commands.spawn((
                    Finish,
                    Collision::Circle(Circle::new(64.)),
                    Speed(0.),
                    SpriteBundle {
                        texture: finish_image.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::splat(64.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(position.extend(1.) * conversion),
                        ..default()
                    },
                    CanHold,
                    RenderLayers::layer(LAYER_INACTIVE),
                ));
            }
            Place::Baton(position) => {
                commands.spawn((
                    Item,
                    Collision::Circle(Circle::new(40.)),
                    SpriteBundle {
                        texture: baton_image.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(128.0, 128.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(position.extend(1.) * conversion),
                        ..default()
                    },
                    RenderLayers::layer(LAYER_INACTIVE),
                ));
            }
        };
    }
}

const CYCLE_COLOR: [Color; 8] = [
    Color::Srgba(GREEN_100),
    Color::Srgba(RED_100),
    Color::Srgba(PURPLE_100),
    Color::Srgba(YELLOW_100),
    Color::Srgba(BLUE_100),
    Color::Srgba(TEAL_100),
    Color::Srgba(ORANGE_100),
    Color::Srgba(PINK_100),
];

fn system_after_images(
    mut index: Local<usize>,
    mut timer: Local<Timer>,
    time: Res<Time>,
    query: Query<(&GlobalTransform, &Speed, Ref<Holding>), With<Active>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let Ok((global_transform, Speed(speed), holding)) = query.get_single() else {
        return;
    };

    let hand_closed = asset_server.load::<Image>("hand-closed.png");
    if holding.0.is_none() {
        return;
    }

    if holding.is_added() && holding.0.is_some() {
        timer.set_duration(Duration::from_millis((10. / speed.abs()) as u64));
        timer.reset();
    };

    timer.tick(time.delta());

    if timer.finished() {
        let mut color = CYCLE_COLOR[*index];
        color.set_alpha(0.7);

        commands.spawn((
            AfterImage,
            FadeOutSpeed(3. * speed.abs()),
            SpriteBundle {
                texture: hand_closed.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(64.0)),
                    color,
                    ..default()
                },
                transform: Transform::from_translation(global_transform.translation()),
                ..default()
            },
        ));

        *index = (*index + 1) % CYCLE_COLOR.len();
        timer.reset();
    }
}

fn fade_out_after_images(
    mut commands: Commands,
    time: Res<Time>,

    mut query: Query<(Entity, &FadeOutSpeed, &mut Sprite), With<AfterImage>>,
) {
    for (entity, FadeOutSpeed(speed), mut sprite) in &mut query {
        let next_alpha = sprite.color.alpha() - speed * time.delta_seconds();
        if let Some(next_size) = sprite.custom_size {
            sprite.custom_size = Some(next_size - Vec2::splat(64. * speed * time.delta_seconds()));
        }
        sprite.color.set_alpha(next_alpha);
        if sprite.color.alpha() <= 0. {
            commands.entity(entity).despawn();
        }
    }
}

fn system_progress(
    mut query: Query<(&mut Progress, &Speed, Option<&Holding>), With<Active>>,
    time: Res<Time>,
) {
    for (mut progress, Speed(speed), holding) in query.iter_mut() {
        progress.0 += match holding {
            Some(Holding(Some(_))) => time.delta_seconds() * speed,
            _ => time.delta_seconds() * 0.5 * speed.signum(), // slower speed to pick up baton again
        };

        if progress.0 > 1. {
            progress.0 = progress.0 - progress.0.trunc();
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

fn system_check_overlap(
    query: Query<(Entity, &GlobalTransform, &Collision)>,
    mut current_overlap: ResMut<Overlap>,
) {
    let mut overlaps = vec![];

    for [(e1, t1, c1), (e2, t2, c2)] in query.iter_combinations() {
        match (c1, c2) {
            (Collision::Circle(c1), Collision::Circle(c2)) => {
                let bc1 = BoundingCircle::new(t1.translation().xy(), c1.radius);
                let bc2 = BoundingCircle::new(t2.translation().xy(), c2.radius);

                if bc1.intersects(&bc2) {
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
    active: Query<(Entity, &Speed, Option<&Holding>), (With<CanHold>, With<Active>)>,
    hand_overs: Query<(Entity, Option<&Speed>), (With<CanHold>, Without<Active>)>,
    mut items: Query<(Entity, &mut Transform), With<Item>>,
    action_input: Res<ActionInput>,
    mut event_writer: EventWriter<GameEvent>,
) {
    if !action_input.just_pressed(Action::Grab) {
        return;
    }

    let Ok((entity, Speed(speed), maybe_holding)) = active.get_single() else {
        return;
    };

    match maybe_holding {
        Some(Holding(Some(item))) => {
            let overlaps = overlap.with(entity);
            let is_overlapping = overlaps.into_iter().find_map(|e| hand_overs.get(e).ok());

            if let Some((other, maybe_speed_other)) = is_overlapping {
                let mut newly_active = commands.entity(other);
                newly_active.insert(Holding(Some(*item)));
                newly_active.insert(Active);
                if let Some(Speed(speed_other)) = maybe_speed_other {
                    newly_active.insert(Speed(speed_other.abs() * -speed.signum()));
                }

                commands.entity(*item).set_parent_in_place(other);

                let mut old_active = commands.entity(entity);
                old_active.remove::<Active>();
                old_active.remove::<Holding>();
                event_writer.send(GameEvent::HandOver);
            } else {
                commands.entity(*item).remove_parent_in_place();
                commands.entity(entity).remove::<Holding>();
                event_writer.send(GameEvent::Drop);
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
                event_writer.send(GameEvent::Grab);
            } else {
                commands.entity(entity).insert(Holding(None));
                event_writer.send(GameEvent::GrabEmpty);
            }
        }
    }
}

fn system_play_sfx(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut events: EventReader<GameEvent>,
) {
    let hand_over = asset_server.load("hand-over.wav");
    let select = asset_server.load("select.wav");
    let select_miss = asset_server.load("select-miss.wav");

    for event in events.read() {
        match event {
            GameEvent::Drop => {
                commands.spawn(AudioBundle {
                    source: select_miss.clone(),
                    ..default()
                });
            }
            GameEvent::HandOver => {
                commands.spawn(AudioBundle {
                    source: hand_over.clone(),
                    ..default()
                });
            }
            GameEvent::Grab => {
                commands.spawn(AudioBundle {
                    source: select.clone(),
                    ..default()
                });
            }
            GameEvent::GrabEmpty => {
                commands.spawn(AudioBundle {
                    source: select_miss.clone(),
                    ..default()
                });
            }
        }
    }
}

fn system_clean_up_sfx(mut commands: Commands, sfxs: Query<(Entity, &AudioSink)>) {
    for (entity, sink) in &sfxs {
        if sink.is_paused() {
            commands.entity(entity).despawn();
        }
    }
}

fn system_lerp_item_to_holding(query: Query<&Holding>, mut items: Query<(&mut Transform, &Item)>) {
    let Ok(Holding(Some(item))) = query.get_single() else {
        return;
    };

    let Ok((mut transform, _)) = items.get_mut(*item) else {
        return;
    };

    transform.translation = transform
        .translation
        .lerp(Vec2::ZERO.extend(transform.translation.z), 0.03);
}

fn system_set_render_layer(
    mut query: Query<(Entity, Option<&Parent>, &mut RenderLayers), With<Active>>,
    mut others: Query<
        &mut RenderLayers,
        (
            Or<(With<Hand>, With<Item>, With<Cycle>, With<Finish>)>,
            Without<Active>,
        ),
    >,
    overlap: Res<Overlap>,
) {
    for mut render_layers in &mut others {
        *render_layers = RenderLayers::layer(LAYER_INACTIVE);
    }

    if let Ok((entity, parent, mut render_layers)) = query.get_single_mut() {
        *render_layers = RenderLayers::layer(LAYER_ACTIVE);

        for other in overlap.with(entity) {
            if let Ok(mut render_layers) = others.get_mut(other) {
                *render_layers = RenderLayers::layer(LAYER_ACTIVE);
            }
        }

        if let Some(mut render_layers) = parent.and_then(|p| others.get_mut(p.get()).ok()) {
            *render_layers = RenderLayers::layer(LAYER_ACTIVE);
        }
    };
}

fn system_tint_layers(mut query: Query<(&mut Sprite, &RenderLayers)>) {
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

fn system_lerp_camera_to_focus(
    focus: Res<CameraFocus>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in &mut query {
        let target = focus.0.extend(transform.translation.z);
        transform.translation = transform.translation.lerp(target, 0.05);
    }
}

fn on_add_active(
    trigger: Trigger<OnAdd, Active>,
    child_query: Query<(&GlobalTransform, Option<&Parent>), With<Active>>,
    parent_query: Query<&GlobalTransform, Without<Active>>,
    mut focus: ResMut<CameraFocus>,
) {
    let Ok((fallback, parent)) = child_query.get(trigger.entity()) else {
        return;
    };

    match parent {
        Some(parent) => {
            let Ok(transform) = parent_query.get(parent.get()) else {
                return;
            };

            focus.0 = transform.translation().xy();
        }
        None => {
            focus.0 = fallback.translation().xy();
        }
    }
}

fn on_remove_grab(
    trigger: Trigger<OnRemove, Holding>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Handle<Image>, With<Hand>>,
) {
    if let Ok(mut sprite) = query.get_mut(trigger.entity()) {
        *sprite = asset_server.load("hand-open.png");
    }
}

fn on_add_grab(
    trigger: Trigger<OnAdd, Holding>,
    asset_server: Res<AssetServer>,
    mut query: Query<&mut Handle<Image>, With<Hand>>,
) {
    if let Ok(mut sprite) = query.get_mut(trigger.entity()) {
        *sprite = asset_server.load("hand-closed.png");
    }
}

fn on_finish(
    trigger: Trigger<OnAdd, Active>,
    mut state: ResMut<NextState<Game>>,
    query: Query<&Finish>,
) {
    if query.get(trigger.entity()).is_ok() {
        state.set(Game::Finished);
    }
}

fn system_fade_out_everything(mut query: Query<&mut Sprite, Without<Item>>) {
    for mut sprite in &mut query {
        let next_alpha = sprite.color.alpha().lerp(0., 0.1);
        sprite.color.set_alpha(next_alpha);
    }
}

fn system_magnify_baton(mut query: Query<&mut Sprite, With<Item>>) {
    for mut sprite in &mut query {
        if let Some(previous_size) = sprite.custom_size {
            let next_size = previous_size.lerp(vec2(256., 256.), 0.1);
            sprite.custom_size = Some(next_size);
        }
    }
}

fn system_show_finish_text(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(10.),
                left: Val::Px(0.),
                right: Val::Px(0.),
                bottom: Val::Px(0.),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle {
                text: Text::from_section(
                    "LIBERTY ACHIEVED",
                    TextStyle {
                        font_size: 64.,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ..default()
            });
        });
}

fn system_play_finish_sound(mut commands: Commands, asset_server: Res<AssetServer>) {
    let finish = asset_server.load("finish.mp3");
    commands.spawn(AudioBundle {
        source: finish.clone(),
        ..default()
    });
}

pub struct GameBundle;

impl Plugin for GameBundle {
    fn build(&self, app: &mut App) {
        app.init_resource::<Overlap>()
            .add_event::<GameEvent>()
            .observe(on_add_active)
            .observe(on_add_grab)
            .observe(on_remove_grab)
            .observe(on_finish)
            .init_resource::<CameraFocus>()
            .insert_state(Game::Playing)
            .add_systems(Startup, system_setup_camera)
            .add_systems(Startup, system_setup_entities)
            .add_systems(PreUpdate, system_check_overlap)
            .add_systems(Update, system_cycle_hand.run_if(in_state(Game::Playing)))
            .add_systems(Update, system_progress.run_if(in_state(Game::Playing)))
            .add_systems(Update, system_grab_toggle.run_if(in_state(Game::Playing)))
            .add_systems(Update, system_tint_layers.run_if(in_state(Game::Playing)))
            .add_systems(Update, system_play_sfx)
            .add_systems(Update, system_lerp_camera_to_focus)
            .add_systems(Update, system_lerp_item_to_holding)
            .add_systems(Update, system_set_render_layer)
            .add_systems(Update, system_after_images)
            .add_systems(Update, fade_out_after_images)
            .add_systems(
                Update,
                system_fade_out_everything.run_if(in_state(Game::Finished)),
            )
            .add_systems(
                Update,
                system_magnify_baton.run_if(in_state(Game::Finished)),
            )
            .add_systems(OnEnter(Game::Finished), system_show_finish_text)
            .add_systems(OnEnter(Game::Finished), system_play_finish_sound)
            .add_systems(PostUpdate, system_clean_up_sfx);
    }
}
