use bevy::prelude::*;

use bevy_ui_navigation::systems::{
    default_gamepad_input, default_keyboard_input, default_mouse_input, InputMapping,
};
use bevy_ui_navigation::{FocusState, Focusable, NavRequestSystem, NavigationPlugin};

/// This example shows the interaction focusables navigation and Ui camera
/// movement.
/// Controls:
/// * `WASD`: move UI camera around,
/// * `QE`: zoom out/zoom in
/// * `IJKL` or arrows: move between focusables
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin)
        // NOTE: The default navigation key mapping can be changed as follow:
        .insert_resource(InputMapping {
            key_up: KeyCode::I,
            key_left: KeyCode::J,
            key_down: KeyCode::K,
            key_right: KeyCode::L,
            ..default()
        })
        .add_startup_system(setup)
        .add_system(cam_system.before(NavRequestSystem))
        .add_system(button_system.after(NavRequestSystem))
        .add_system(default_keyboard_input.before(NavRequestSystem))
        .add_system(default_gamepad_input.before(NavRequestSystem))
        .add_system(default_mouse_input.before(NavRequestSystem))
        .run();
}

#[derive(Component)]
struct IdleColor(UiColor);

fn cam_system(
    mut cam: Query<(&mut Transform, &mut OrthographicProjection), With<CameraUi>>,
    input: Res<Input<KeyCode>>,
) {
    let (mut cam_trans, mut cam_proj) = cam.single_mut();
    let offset = match () {
        () if input.pressed(KeyCode::A) => -Vec3::X,
        () if input.pressed(KeyCode::D) => Vec3::X,
        () if input.pressed(KeyCode::S) => -Vec3::Y,
        () if input.pressed(KeyCode::W) => Vec3::Y,
        () => Vec3::ZERO,
    };
    // We only modify the transform when there is a change, so as
    // to not trigger change detection.
    if offset != Vec3::ZERO {
        cam_trans.translation += offset * cam_proj.scale * 30.0;
    }
    let scale_offset = match () {
        () if input.pressed(KeyCode::Q) => 0.9,
        () if input.pressed(KeyCode::E) => 1.1,
        () => 0.0,
    };
    if scale_offset != 0.0 {
        cam_proj.scale *= scale_offset;
    }
}

fn button_system(
    mut interaction_query: Query<(&Focusable, &mut UiColor, &IdleColor), Changed<Focusable>>,
) {
    for (focusable, mut material, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if let FocusState::Focused = focusable.state() {
            *material = Color::WHITE.into();
        } else {
            *material = *idle_color;
        }
    }
}

fn setup(mut commands: Commands) {
    let top = 30;
    let as_rainbow = |i: u32| Color::hsl((i as f32 / top as f32) * 360.0, 0.9, 0.5);
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    for i in 0..top {
        for j in 0..top {
            let full = (i + j).max(1);
            spawn_button(&mut commands, as_rainbow((i * j) % full).into(), top, i, j);
        }
    }
}
fn spawn_button(commands: &mut Commands, color: UiColor, max: u32, i: u32, j: u32) {
    let size = 340.0 / max as f32;
    commands
        .spawn_bundle(ButtonBundle {
            color,
            style: Style {
                size: Size::new(Val::Percent(size), Val::Percent(size)),
                position_type: PositionType::Absolute,
                position: Rect {
                    bottom: Val::Percent((400.0 / max as f32) * i as f32),
                    left: Val::Percent((400.0 / max as f32) * j as f32),
                    ..default()
                },
                ..default()
            },
            ..default()
        })
        .insert(Focusable::default())
        .insert(IdleColor(color));
}
