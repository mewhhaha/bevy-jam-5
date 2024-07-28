use bevy::color::palettes::tailwind::GREEN_600;
use bevy::math::vec2;
use bevy::prelude::*;

use crate::game::Collision;
use crate::input::{Action, ActionInput};

fn debug_gizmo_grid(mut show: Local<bool>, action_input: Res<ActionInput>, mut gizmos: Gizmos) {
    if action_input.just_pressed(Action::DebugShowCollisions) {
        *show = !*show;
    }

    if !*show {
        return;
    }

    gizmos.grid_2d(
        Vec2::ZERO,
        0.,
        UVec2 { x: 100, y: 100 },
        vec2(64., 64.),
        Color::linear_rgb(0.2, 0.2, 0.2),
    );
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
            Collision::Circle(circle) => {
                gizmos.primitive_2d(circle, translation, 0., GREEN_600);
            }
        }
    }
}

pub struct DebugBundle;

impl Plugin for DebugBundle {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, debug_show_collision_gizmos)
            .add_systems(PostUpdate, debug_gizmo_grid);
    }
}
