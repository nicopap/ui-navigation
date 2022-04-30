use bevy::prelude::*;

use bevy_ui_navigation::systems::{
    default_gamepad_input, default_keyboard_input, default_mouse_input, InputMapping,
};
use bevy_ui_navigation::{FocusState, Focusable, NavRequestSystem, NavigationPlugin};

/// This example shows what happens when there is a lot of focusables on screen.
/// It doesn't run well on debug builds, you should try running it with the `--release`
/// flag.
///
/// It is very useful to assess the performance of bevy ui and how expansive our systems
/// are.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin)
        .init_resource::<InputMapping>()
        .add_startup_system(setup)
        .add_system(button_system.after(NavRequestSystem))
        .add_system(default_keyboard_input.before(NavRequestSystem))
        .add_system(default_gamepad_input.before(NavRequestSystem))
        .add_system(default_mouse_input.before(NavRequestSystem))
        .run();
}

#[derive(Component)]
struct IdleColor(UiColor);

fn button_system(
    mut interaction_query: Query<(&Focusable, &mut UiColor, &IdleColor), Changed<Focusable>>,
) {
    for (focusable, mut material, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if let FocusState::Focused = focusable.state() {
            *material = Color::ORANGE_RED.into();
        } else {
            *material = *idle_color;
        }
    }
}

fn setup(mut commands: Commands) {
    let top = 310;
    let as_rainbow = |i: u32| Color::hsl((i as f32 / top as f32) * 360.0, 0.9, 0.8);
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                // position_type: PositionType::Absolute,
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for i in 0..top {
                for j in 0..top {
                    spawn_button(commands, as_rainbow(j % i.max(1)).into(), top, i, j);
                }
            }
        });
}
fn spawn_button(commands: &mut ChildBuilder, color: UiColor, max: u32, i: u32, j: u32) {
    let width = 90.0 / max as f32;
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Percent(width), Val::Percent(width)),

                position: Rect {
                    bottom: Val::Percent((100.0 / max as f32) * i as f32),
                    left: Val::Percent((100.0 / max as f32) * j as f32),
                    ..default()
                },
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            color,
            ..Default::default()
        })
        .insert(Focusable::default())
        .insert(IdleColor(color));
}
