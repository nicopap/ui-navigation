use bevy::prelude::*;

use bevy_ui_navigation::{
    prelude::{DefaultNavigationPlugins, FocusState, Focusable, NavEvent, NavRequestSystem},
    systems::InputMapping,
};

/// This example illustrates how to mark buttons as focusable and let
/// NavigationPlugin figure out how to go from one to another.
/// See lines 15 and 89 for details.
fn main() {
    App::new()
        // 1: Add the DefaultNavigationPlugins
        //                            vvvvvvvvvvvvvvvvvvvvvvvv
        .add_plugins((DefaultPlugins, DefaultNavigationPlugins))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // So that the UI _feels_ smooth, make sure to update the visual
                // after the navigation system ran
                button_system.after(NavRequestSystem),
                print_nav_events.after(NavRequestSystem),
            ),
        )
        .run();
}

fn button_system(
    mut interaction_query: Query<(&Focusable, &mut BackgroundColor), Changed<Focusable>>,
) {
    for (focusable, mut material) in interaction_query.iter_mut() {
        if let FocusState::Focused = focusable.state() {
            *material = Color::ORANGE_RED.into();
        } else {
            *material = Color::DARK_GRAY.into();
        }
    }
}
fn print_nav_events(mut events: EventReader<NavEvent>) {
    for event in events.iter() {
        println!("{:?}", event);
    }
}

fn setup(mut commands: Commands, mut input_mapping: ResMut<InputMapping>) {
    input_mapping.keyboard_navigation = true;
    input_mapping.focus_follows_mouse = true;
    // ui camera
    commands.spawn(Camera2dBundle::default());
    let positions = [
        Vec2::new(10.0, 10.0),
        Vec2::new(15.0, 50.0),
        Vec2::new(20.0, 90.0),
        Vec2::new(30.0, 10.0),
        Vec2::new(35.0, 50.0),
        Vec2::new(40.0, 90.0),
        Vec2::new(60.0, 10.0),
        Vec2::new(55.0, 50.0),
        Vec2::new(50.0, 90.0),
    ];
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for pos in positions {
                spawn_button(pos, commands);
            }
        });
}
fn spawn_button(position: Vec2, commands: &mut ChildBuilder) {
    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(95.0),
                height: Val::Px(65.0),
                left: Val::Percent(position.x),
                top: Val::Percent(position.y),
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            background_color: Color::DARK_GRAY.into(),
            ..Default::default()
        },
        // vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
        // 2. Add the `Focusable` component to the navigable Entity
        Focusable::default(),
    ));
}
