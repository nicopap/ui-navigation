use bevy::input::{keyboard::KeyboardInput, ElementState};
use bevy::prelude::*;

use bevy_ui_navigation::{Direction, Focusable, NavEvent, NavRequest, NavigationPlugin};

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
        .add_system(print_nav_events)
        .run();
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    focused: Handle<ColorMaterial>,
    active: Handle<ColorMaterial>,
}

impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::DARK_GRAY.into()),
            focused: materials.add(Color::ORANGE_RED.into()),
            active: materials.add(Color::GOLD.into()),
        }
    }
}

fn keyboard_input(mut keyboard: EventReader<KeyboardInput>, mut nav_cmds: EventWriter<NavRequest>) {
    use Direction::*;
    use NavRequest::*;
    let command_mapping = |code| match code {
        KeyCode::Return => Some(Action),
        KeyCode::Back => Some(Cancel),
        KeyCode::Up => Some(Move(North)),
        KeyCode::Down => Some(Move(South)),
        KeyCode::Left => Some(Move(West)),
        KeyCode::Right => Some(Move(East)),
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
    mut interaction_query: Query<
        (&Focusable, &mut Handle<ColorMaterial>),
        (Changed<Focusable>, With<Button>),
    >,
) {
    for (focus_state, mut material) in interaction_query.iter_mut() {
        if focus_state.is_focused() {
            *material = button_materials.focused.clone();
        } else if focus_state.is_active() {
            *material = button_materials.active.clone();
        } else {
            *material = button_materials.normal.clone();
        }
    }
}
fn print_nav_events(mut events: EventReader<NavEvent>) {
    for event in events.iter() {
        println!("{:?}", event);
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
        .insert(Focusable::default());
}
