use bevy::input::{keyboard::KeyboardInput, ElementState};
use bevy::prelude::*;

use bevy_ui_navigation::{
    Direction, Focusable, Focused, NavEvent, NavFence, NavRequest, NavigationPlugin,
};

/// Shows how navigation is supported even between siblings separated by a
/// hierahierarchical level of nodes, shows how to "wall of" a part of the UI
/// (so that it requires different interactions to reach)
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin)
        .init_resource::<ButtonMaterials>()
        .add_startup_system(setup)
        .add_system(button_system)
        .add_system(print_nav_events)
        .add_system(keyboard_input)
        .run();
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    focused: Handle<ColorMaterial>,
    pink: Handle<ColorMaterial>,
    backgrounds: [Handle<ColorMaterial>; 3],
}

impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
            focused: materials.add(Color::rgb(0.35, 0.75, 0.35).into()),
            pink: materials.add(Color::rgba(1.00, 0.35, 1.0, 0.5).into()),
            backgrounds: [
                materials.add(Color::rgba(1.0, 0.35, 0.35, 0.5).into()),
                materials.add(Color::rgba(0.35, 1.0, 0.35, 0.5).into()),
                materials.add(Color::rgba(0.35, 0.35, 1.0, 0.5).into()),
            ],
        }
    }
}

fn print_nav_events(mut events: EventReader<NavEvent>) {
    for event in events.iter() {
        println!("{:?}", event);
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

fn setup(mut commands: Commands, button_materials: Res<ButtonMaterials>) {
    let size = |width, height| Size::new(Val::Percent(width), Val::Percent(height));
    let flex_wrap = FlexWrap::Wrap;
    let style = Style {
        size: size(100.0, 100.0),
        flex_wrap,
        ..Style::default()
    };
    let bundle = NodeBundle {
        style,
        ..Default::default()
    };
    let size = size(45.0, 45.0);
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(bundle)
        // The `Focusable`s buttons are not direct siblings, we can navigate through
        // them beyond direct hierarchical relationships.
        //
        // To prevent this, we can add a `NavFence` as a sort of boundary
        // between different sets of `Focusable`s. This requires having an
        // englobing `NavFence` that contains all other `NavFence`s or
        // `Focusable`s
        //
        // YOU MUSTE ADD A NavFence enclosing ALL Focusable and ALL NavFence (but
        // themselves) Subtile broken behavior will ensure otherwise
        .insert(NavFence::root())
        .with_children(|commands| {
            for i in 0..3 {
                let style = Style {
                    size,
                    ..Style::default()
                };
                let bundle = NodeBundle {
                    style,
                    material: button_materials.backgrounds[i].clone(),
                    ..Default::default()
                };
                commands.spawn_bundle(bundle).with_children(|commands| {
                    spawn_button(commands, &button_materials);
                    spawn_button(commands, &button_materials);
                    spawn_button(commands, &button_materials);
                });
            }
            let style = Style {
                size,
                ..Style::default()
            };
            let bundle = NodeBundle {
                style,
                material: button_materials.pink.clone(),
                ..Default::default()
            };
            commands
                .spawn_bundle(bundle)
                // We don't want to be able to access the pink square, so we
                // add a `NavFence` as boundary
                .insert(NavFence::root())
                .with_children(|commands| {
                    spawn_button(commands, &button_materials);
                    spawn_button(commands, &button_materials);
                    spawn_button(commands, &button_materials);
                    spawn_button(commands, &button_materials);
                });
        });
}
fn spawn_button(commands: &mut ChildBuilder, button_materials: &ButtonMaterials) {
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(120.0), Val::Px(60.0)),
                margin: Rect::all(Val::Percent(4.0)),
                ..Default::default()
            },
            material: button_materials.normal.clone(),
            ..Default::default()
        })
        // The `Focusable`s are not direct siblings, we can navigate through
        // them beyond direct hierarchical relationships.
        .insert(Focusable::default());
}
