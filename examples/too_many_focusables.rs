use bevy::prelude::*;

use bevy_ui_navigation::events::{Direction, NavRequest};
use bevy_ui_navigation::prelude::{
    DefaultNavigationPlugins, FocusState, Focusable, NavRequestSystem,
};
use bevy_ui_navigation::systems::InputMapping;

/// This example shows what happens when there is a lot of focusables on screen.
/// It doesn't run well on debug builds, you should try running it with the `--release`
/// flag.
///
/// It is very useful to assess the performance of bevy ui and how expansive our systems
/// are.
///
/// You can toggle automatic generation of NavRequest with the `K` key.
fn main() {
    App::new()
        .add_plugins((DefaultPlugins, DefaultNavigationPlugins))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                non_stop_move.before(NavRequestSystem),
                button_system.after(NavRequestSystem),
            ),
        )
        .run();
}

#[derive(Component)]
struct IdleColor(BackgroundColor);

fn button_system(
    mut interaction_query: Query<
        (&Focusable, &mut BackgroundColor, &IdleColor),
        Changed<Focusable>,
    >,
) {
    for (focusable, mut material, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if let FocusState::Focused = focusable.state() {
            *material = Color::ORANGE_RED.into();
        } else {
            *material = *idle_color;
        }
    }
}

struct MyDirection(Direction);
impl Default for MyDirection {
    fn default() -> Self {
        Self(Direction::South)
    }
}

fn non_stop_move(
    input: Res<Input<KeyCode>>,
    mut requests: EventWriter<NavRequest>,
    mut enabled: Local<bool>,
    time: Res<Time>,
    mut last_direction: Local<MyDirection>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;
    if input.just_pressed(KeyCode::K) {
        *enabled = !*enabled;
    }
    if *enabled {
        for _ in 0..10 {
            requests.send(NavRequest::Move(last_direction.0));
        }
    }
    if at_interval(2.0) {
        let new_direction = match last_direction.0 {
            Direction::East => Direction::North,
            Direction::North => Direction::West,
            Direction::West => Direction::South,
            Direction::South => Direction::East,
        };
        last_direction.0 = new_direction;
    }
}

fn setup(mut commands: Commands, mut input_mapping: ResMut<InputMapping>) {
    use Val::Percent as Pct;
    input_mapping.keyboard_navigation = true;
    input_mapping.focus_follows_mouse = true;
    let top = 310;
    let as_rainbow = |i: u32| Color::hsl((i as f32 / top as f32) * 360.0, 0.9, 0.8);
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                // position_type: PositionType::Absolute,
                width: Pct(100.),
                height: Pct(100.),
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
fn spawn_button(commands: &mut ChildBuilder, color: BackgroundColor, max: u32, i: u32, j: u32) {
    use Val::Percent as Pct;
    let width = 90.0 / max as f32;
    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Pct(width),
                height: Pct(width),
                bottom: Pct((100.0 / max as f32) * i as f32),
                left: Pct((100.0 / max as f32) * j as f32),
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            background_color: color,
            ..Default::default()
        },
        Focusable::default(),
        IdleColor(color),
    ));
}
