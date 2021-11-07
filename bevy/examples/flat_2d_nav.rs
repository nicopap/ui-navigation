use bevy::input::{keyboard::KeyboardInput, ElementState};
use bevy::prelude::*;

use bevy_ui_navigation::{Focusable, Focused, NavCommand, NavigationPlugin};

/// This example illustrates how to mark buttons as focusable and let
/// NavigationPlugin figure out how to go from one to another.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // 1: Add the NavigationPlugin
        .add_plugin(NavigationPlugin)
        .init_resource::<ButtonMaterials>()
        .add_startup_system(setup)
        .add_system(button_system)
        .add_system(keyboard_input)
        .run();
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    focused: Handle<ColorMaterial>,
}

impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
            focused: materials.add(Color::rgb(0.35, 0.75, 0.35).into()),
        }
    }
}

fn keyboard_input(mut keyboard: EventReader<KeyboardInput>, mut nav_cmds: EventWriter<NavCommand>) {
    use NavCommand::*;
    let command_mapping = |code: KeyCode| match code {
        KeyCode::Return => Some(Action),
        KeyCode::Up => Some(MoveUp),
        KeyCode::Down => Some(MoveDown),
        KeyCode::Left => Some(MoveLeft),
        KeyCode::Right => Some(MoveRight),
        _ => None,
    };
    for event in keyboard.iter() {
        if event.state == ElementState::Released {
            if let Some(cmd) = event.key_code.and_then(command_mapping) {
                nav_cmds.send(cmd)
            }
        }
    }
}

fn button_system(
    button_materials: Res<ButtonMaterials>,
    // I'm considering a system where it is less cumbersome to check for focus
    // (I think I'll add `focused` and `active` fields to `Focusable`)
    mut interaction_query: Query<(Option<&Focused>, &mut Handle<ColorMaterial>), With<Button>>,
) {
    for (interaction, mut material) in interaction_query.iter_mut() {
        match interaction {
            Some(_) => {
                *material = button_materials.focused.clone();
            }
            None => {
                *material = button_materials.normal.clone();
            }
        }
    }
}

macro_rules! xy {
    ($x:expr, $y:expr) => {
        Vec2::new($x as f32, $y as f32)
    };
}
fn setup(mut commands: Commands, button_materials: Res<ButtonMaterials>) {
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    let positions = [
        xy!(10, 10),
        xy!(15, 50),
        xy!(20, 90),
        xy!(30, 10),
        xy!(35, 50),
        xy!(40, 90),
        xy!(60, 10),
        xy!(55, 50),
        xy!(50, 90),
    ];
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for pos in positions {
                spawn_button(pos, commands, &button_materials);
            }
        });
}
fn spawn_button(position: Vec2, commands: &mut ChildBuilder, button_materials: &ButtonMaterials) {
    let position = Rect {
        left: Val::Percent(position.x),
        top: Val::Percent(position.y),
        ..Default::default()
    };
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(95.0), Val::Px(65.0)),
                position,
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            material: button_materials.normal.clone(),
            ..Default::default()
        })
        // 2. Add the `Focusable` component to the navigable Entity
        .insert(Focusable);
}
