use bevy::prelude::*;

use bevy::input::{keyboard::KeyboardInput, ElementState};
use bevy_ui_build_macros::{build_ui, rect, size, style, unit};
use bevy_ui_navigation::{
    Direction, Focusable, MenuDirection, NavFence, NavRequest, NavigationPlugin,
};

/// THE ULTIMATE MENU DEMONSTRATION
///
/// This is an unrealistic menu demonstrating tabbed navigation, focus memory
/// and navigation hierarchy traversal. It is similar to your classical RPG
/// menu, with the significant difference that **all tabs are shown at the same
/// time on screen** rather than hidden and shown as the tabs are selected.
///
/// The use of macros is not _needed_ but extremely useful. Removes the noise
/// from the ui declaration and helps focus the example on the important stuff,
/// not the UI building boilerplate.
///
/// Use `Q` and `E` to navigate tabs, use `WASD` for moving within containers,
/// `ENTER` and `BACKSPACE` for going down/up the hierarchy.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Materials>()
        .insert_resource(Lapse(0.0))
        .add_plugin(NavigationPlugin)
        .add_startup_system(setup)
        .add_system(button_system)
        .add_system(gamepad_input)
        .add_system(keyboard_input)
        .run();
}

struct Lapse(f64);

#[derive(Default, Clone, Bundle)]
struct FocusableButtonBundle {
    #[bundle]
    button_bundle: ButtonBundle,
    focus: Focusable,
}

struct Materials {
    inert: Handle<ColorMaterial>,
    focused: Handle<ColorMaterial>,
    active: Handle<ColorMaterial>,
    dormant: Handle<ColorMaterial>,
}
impl FromWorld for Materials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        Materials {
            inert: materials.add(Color::DARK_GRAY.into()),
            focused: materials.add(Color::ORANGE_RED.into()),
            active: materials.add(Color::GOLD.into()),
            dormant: materials.add(Color::GRAY.into()),
        }
    }
}

fn gamepad_input(
    mut events: EventReader<GamepadEvent>,
    mut nav_cmds: EventWriter<NavRequest>,
    mut lapse: ResMut<Lapse>,
    time: Res<Time>,
) {
    use Direction::*;
    use MenuDirection::{Previous, Next};
    use NavRequest::{MenuMove, Action, Cancel, Move};
    use GamepadAxisType::{DPadX, DPadY};
    use GamepadButtonType as Butt;

    let hat_to_dir = |axis: GamepadAxisType, value: f32| {
        match () {
            () if axis == DPadX && value > 0.0 => Some(East),
            () if axis == DPadX && value < 0.0 => Some(West),
            () if axis == DPadY && value > 0.0 => Some(North),
            () if axis == DPadY && value < 0.0 => Some(South),
            () => None,
        }
    };
    let button_to_request = |button: GamepadButtonType| {
        match () {
            () if button == Butt::South => Some(Action),
            () if button == Butt::East => Some(Cancel),
            () if button == Butt::LeftTrigger => Some(MenuMove(Previous)),
            () if button == Butt::RightTrigger => Some(MenuMove(Next)),
            () => None,
        }
    };
    for event in events.iter() {
        let maybe_cmd = match &event {
            GamepadEvent(Gamepad(0), GamepadEventType::AxisChanged(axis, value)) =>
                hat_to_dir(*axis, *value).map(|dir| Move(dir)),
            GamepadEvent(Gamepad(0), GamepadEventType::ButtonChanged(button, v)) if v > &0.0 =>
                button_to_request(*button),
            _ => None
        };
        if let Some(cmd) = maybe_cmd {
            lapse.0 = time.seconds_since_startup();
            nav_cmds.send(cmd)
        }
    }
}

fn keyboard_input(
    mut keyboard: EventReader<KeyboardInput>,
    mut nav_cmds: EventWriter<NavRequest>,
    mut lapse: ResMut<Lapse>,
    time: Res<Time>,
) {
    use Direction::*;
    use NavRequest::*;
    let command_mapping = |code| match code {
        KeyCode::Return => Some(Action),
        KeyCode::Back => Some(Cancel),
        KeyCode::Up | KeyCode::W => Some(Move(North)),
        KeyCode::Down | KeyCode::S => Some(Move(South)),
        KeyCode::Left | KeyCode::A => Some(Move(West)),
        KeyCode::Right | KeyCode::D => Some(Move(East)),
        KeyCode::Tab | KeyCode::E => Some(MenuMove(MenuDirection::Next)),
        KeyCode::Q => Some(MenuMove(MenuDirection::Previous)),
        _ => None,
    };
    for event in keyboard.iter() {
        if event.state == ElementState::Released {
            if let Some(cmd) = event.key_code.and_then(command_mapping) {
                lapse.0 = time.seconds_since_startup();
                nav_cmds.send(cmd)
            }
        }
    }
}

fn button_system(
    materials: Res<Materials>,
    lapse: Res<Lapse>,
    time: Res<Time>,
    mut interaction_query: Query<
        (&Focusable, &mut Handle<ColorMaterial>),
        (Changed<Focusable>, With<Button>),
    >,
) {
    let mut already_displayed = false;
    for (focus_state, mut material) in interaction_query.iter_mut() {
        if !already_displayed {
            println!(
                "request handling time: {:.3}",
                time.seconds_since_startup() - lapse.0
            );
            already_displayed = true;
        }
        if focus_state.is_focused() {
            *material = materials.focused.clone();
        } else if focus_state.is_active() {
            *material = materials.active.clone();
        } else if focus_state.is_dormant() {
            *material = materials.dormant.clone();
        } else {
            *material = materials.inert.clone();
        }
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    our_materials: Res<Materials>,
) {
    use FlexDirection::{ColumnReverse, Row};
    use FlexWrap::Wrap;
    use JustifyContent::{FlexStart, SpaceBetween};
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());

    let vertical = NodeBundle {
        style: style! {
            flex_direction: ColumnReverse,
            size: size!(100 pct, 100 pct),
            margin: rect!(2 px),
        },
        material: materials.add(Color::NONE.into()),
        ..Default::default()
    };
    let horizontal = NodeBundle {
        style: style! {
            flex_direction: Row,
            size: size!(100 pct, 100 pct),
            justify_content: SpaceBetween,
            margin: rect!(2 px),
        },
        material: materials.add(Color::NONE.into()),
        ..Default::default()
    };
    let red = materials.add(Color::RED.into());
    let blue = materials.add(Color::BLUE.into());
    let green = materials.add(Color::GREEN.into());
    let gray = materials.add(Color::rgba(0.9, 0.9, 0.9, 0.3).into());
    let black = our_materials.inert.clone();

    let square = FocusableButtonBundle {
        button_bundle: ButtonBundle {
            style: style! {
                size: size!(40 px, 40 px),
                margin: rect!(2 px),
            },
            material: black.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    let select_square = FocusableButtonBundle {
        button_bundle: ButtonBundle {
            style: style! {
                size: size!(100 pct, 40 px),
                margin: rect!(2 px),
            },
            material: black.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    let tab_square = FocusableButtonBundle {
        button_bundle: ButtonBundle {
            style: style! {
                size: size!(100 px, 40 px),
                margin: rect!(30 px, 0 px),
            },
            material: black,
            ..Default::default()
        },
        ..Default::default()
    };
    let column_box = NodeBundle {
        style: style! {
            flex_direction: Row,
            flex_basis: unit!(90 pct),
            size: size!(100 pct, 90 pct),
            padding: rect!(30 px),
        },
        ..Default::default()
    };
    let column = NodeBundle {
        style: style! {
            flex_direction: ColumnReverse,
            size: size!(33 pct, 100 pct),
            padding: rect!(10 px),
            margin: rect!(5 px, 0 px),
        },
        ..Default::default()
    };
    let colored_square = NodeBundle {
        style: style! { size: size!(100 pct, 100 pct), },
        material: materials.add(Color::rgb(1.0, 0.3, 0.9).into()),
        ..Default::default()
    };

    let fence = |id: Entity| NavFence::reachable_from(id);
    let loop_fence = |id: Entity| NavFence::reachable_from(id).looping();
    let mut spawn = |bundle: &FocusableButtonBundle| commands.spawn_bundle(bundle.clone()).id();

    let tab_red = spawn(&tab_square);
    let tab_green = spawn(&tab_square);
    let tab_blue = spawn(&tab_square);

    let select_1 = spawn(&select_square);
    let select_2 = spawn(&select_square);

    let g1 = spawn(&select_square);
    let g2 = spawn(&select_square);
    let g3 = spawn(&select_square);
    let g4 = spawn(&select_square);
    let g5 = spawn(&select_square);
    let g6 = spawn(&select_square);
    let g7 = spawn(&select_square);
    let g8 = spawn(&select_square);

    // The macro is a very thin wrapper over the "normal" UI declaration
    // technic. Please look at the doc for `build_ui` for info on what it does.
    //
    // Pay attention to calls to `focus()`, `fence(id)` and `NavFence::root()`
    build_ui! {
        #[cmd(commands)]
        // The tab menu should be navigated with `MenuDirection::{Next, Previous}`
        // hence the `.sequence()`
        vertical{size:size!(100 pct, 100 pct)}[NavFence::root().looping().sequence()](
            horizontal{justify_content: FlexStart, flex_basis: unit!(10 pct)}(
                // tab_{red,green,blue} link to their respective columns
                // vvvvvvv      vvvvvvvvv      vvvvvvvv
                id(tab_red), id(tab_green), id(tab_blue)
            ),
            column_box(
                //          vvvvvvvvvvvvvv
                column[red, fence(tab_red)](
                    vertical(id(select_1), id(select_2)),
                    horizontal{flex_wrap: Wrap}[gray, loop_fence(select_1)](
                        square, square, square, square, square, square, square, square,
                        square, square, square, square, square, square, square, square,
                        square, square, square, square
                    ),
                    horizontal{flex_wrap: Wrap}[gray, loop_fence(select_2)](
                        square, square, square, square, square, square, square, square
                    )
                ),
                //            vvvvvvvvvvvvvvvv
                column[green, fence(tab_green)](
                    horizontal(id(g1), horizontal[gray, fence(g1)](square)),
                    horizontal(id(g2), horizontal[gray, loop_fence(g2)](square, square)),
                    horizontal(id(g3), horizontal[gray, fence(g3)](square, square, square)),
                    horizontal(id(g4), horizontal[gray, loop_fence(g4)](square)),
                    horizontal(id(g5), horizontal[gray, fence(g5)](square, square, square)),
                    horizontal(id(g6), horizontal[gray, loop_fence(g6)](square, square)),
                    horizontal(id(g7), horizontal[gray, fence(g7)](square, square, square, square)),
                    horizontal(id(g8), horizontal[gray, loop_fence(g8)](square, square, square, square))
                ),
                //           vvvvvvvvvvvvvvv
                column[blue, fence(tab_blue)](
                    vertical(
                        vertical(select_square, select_square, select_square, select_square),
                        colored_square
                    )
                )
            )
        )
    };
}
