use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Grab,
    DebugShowCollisions,
}

struct ActionState {
    action: Action,
    pressed: bool,
    just_pressed: bool,
    just_released: bool,
}

impl ActionState {
    fn read((action, key): (Action, KeyCode), input: &ButtonInput<KeyCode>) -> ActionState {
        Self {
            action,
            pressed: input.pressed(key),
            just_pressed: input.just_pressed(key),
            just_released: input.just_released(key),
        }
    }
}

impl Action {
    fn state(self) -> ActionState {
        ActionState {
            action: self,
            pressed: false,
            just_pressed: false,
            just_released: false,
        }
    }
}

#[derive(Resource)]
pub struct ActionInput([ActionState; 2]);

impl Default for ActionInput {
    fn default() -> Self {
        Self([Action::Grab.state(), Action::DebugShowCollisions.state()])
    }
}

impl ActionInput {
    pub fn just_pressed(&self, action: Action) -> bool {
        self.0
            .iter()
            .find(|state| state.action == action)
            .map(|state| state.just_pressed)
            .unwrap_or(false)
    }

    pub fn pressed(&self, action: Action) -> bool {
        self.0
            .iter()
            .find(|state| state.action == action)
            .map(|state| state.pressed)
            .unwrap_or(false)
    }

    pub fn just_released(&self, action: Action) -> bool {
        self.0
            .iter()
            .find(|state| state.action == action)
            .map(|state| state.just_released)
            .unwrap_or(false)
    }
}

fn read_input(buttons: Res<ButtonInput<KeyCode>>, mut action_input: ResMut<ActionInput>) {
    let mappings = [
        (Action::Grab, KeyCode::Space),
        (Action::DebugShowCollisions, KeyCode::KeyD),
    ];

    let actions = mappings.map(|mapping| ActionState::read(mapping, &buttons));
    action_input.0 = actions;
}

pub struct InputMappingBundle;

impl Plugin for InputMappingBundle {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionInput>()
            .add_systems(PreUpdate, read_input);
    }
}
