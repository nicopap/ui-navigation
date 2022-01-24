use bevy::prelude::*;

use bevy_ui_build_macros::{build_ui, rect, size, style, unit};
use bevy_ui_navigation::{
    components::FocusableButtonBundle,
    systems::{default_gamepad_input, default_keyboard_input, default_mouse_input, InputMapping},
    Focusable, NavMenu, NavigationPlugin,
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
///
/// Navigation also works with controller
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Materials>()
        .init_resource::<InputMapping>()
        .add_plugin(NavigationPlugin)
        .add_startup_system(setup)
        .add_system(button_system)
        .add_system(default_gamepad_input)
        .add_system(default_keyboard_input)
        .add_system(default_mouse_input)
        .run();
}

struct Materials {
    inert: Color,
    focused: Color,
    active: Color,
    dormant: Color,
}
impl Default for Materials {
    fn default() -> Self {
        Materials {
            inert: Color::DARK_GRAY,
            focused: Color::ORANGE_RED,
            active: Color::GOLD,
            dormant: Color::GRAY,
        }
    }
}

#[allow(clippy::type_complexity)]
fn button_system(
    materials: Res<Materials>,
    mut interaction_query: Query<(&Focusable, &mut UiColor), (Changed<Focusable>, With<Button>)>,
) {
    for (focus_state, mut material) in interaction_query.iter_mut() {
        if focus_state.is_focused() {
            *material = materials.focused.into();
        } else if focus_state.is_active() {
            *material = materials.active.into();
        } else if focus_state.is_dormant() {
            *material = materials.dormant.into();
        } else {
            *material = materials.inert.into();
        }
    }
}

fn setup(mut commands: Commands, our_materials: Res<Materials>) {
    use FlexDirection::{ColumnReverse, Row};
    use FlexWrap::Wrap;
    use JustifyContent::{FlexStart, SpaceBetween};
    // ui camera
    commands
        .spawn_bundle(UiCameraBundle::default())
        .insert(Transform::from_xyz(40.0, -60.0, 1000.0 - 0.1));

    let red: UiColor = Color::RED.into();
    let blue: UiColor = Color::BLUE.into();
    let green: UiColor = Color::GREEN.into();
    let gray: UiColor = Color::rgba(0.9, 0.9, 0.9, 0.3).into();
    let black: UiColor = our_materials.inert.into();
    let transparent: UiColor = Color::NONE.into();

    let vertical = NodeBundle {
        style: style! {
            flex_direction: ColumnReverse,
            size: size!(100 pct, 100 pct),
            margin: rect!(2 px),
        },
        color: transparent,
        ..Default::default()
    };
    let horizontal = NodeBundle {
        style: style! {
            flex_direction: Row,
            size: size!(100 pct, 100 pct),
            justify_content: SpaceBetween,
            margin: rect!(2 px),
        },
        color: transparent,
        ..Default::default()
    };
    let square = FocusableButtonBundle::from(ButtonBundle {
        style: style! {
            size: size!(40 px, 40 px),
            margin: rect!(2 px),
        },
        color: black,
        ..Default::default()
    });
    let select_square = FocusableButtonBundle::from(ButtonBundle {
        style: style! {
            size: size!(100 pct, 40 px),
            margin: rect!(2 px),
        },
        color: black,
        ..Default::default()
    });
    let tab_square = FocusableButtonBundle::from(ButtonBundle {
        style: style! {
            size: size!(100 px, 40 px),
            margin: rect!(30 px, 0 px),
        },
        color: black,
        ..Default::default()
    });
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
        color: Color::rgb(1.0, 0.3, 0.9).into(),
        ..Default::default()
    };

    let menu = NavMenu::reachable_from;
    let cycle_menu = |id: Entity| NavMenu::reachable_from(id).cycling();
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
    // Pay attention to calls to `menu(id)` and `NavMenu::root()`
    build_ui! {
        #[cmd(commands)]
        // The tab menu should be navigated with `NavRequest::ScopeMove`
        // hence the `.scope()`
        vertical{size:size!(100 pct, 100 pct)}[;NavMenu::root().cycling().scope()](
            horizontal{justify_content: FlexStart, flex_basis: unit!(10 pct)}(
                // tab_{red,green,blue} link to their respective columns
                // vvvvvvv      vvvvvvvvv      vvvvvvvv
                id(tab_red), id(tab_green), id(tab_blue)
            ),
            column_box(
                //          vvvvvvvvvvvvvv
                column[;red, menu(tab_red)](
                    vertical(id(select_1), id(select_2)),
                    horizontal{flex_wrap: Wrap}[;gray, cycle_menu(select_1)](
                        square, square, square, square, square, square, square, square,
                        square, square, square, square, square, square, square, square,
                        square, square, square, square
                    ),
                    horizontal{flex_wrap: Wrap}[;gray, cycle_menu(select_2)](
                        square, square, square, square, square, square, square, square
                    )
                ),
                //            vvvvvvvvvvvvvvvv
                column[;green, menu(tab_green)](
                    horizontal(id(g1), horizontal[;gray, menu(g1)](square)),
                    horizontal(id(g2), horizontal[;gray, cycle_menu(g2)](square, square)),
                    horizontal(id(g3), horizontal[;gray, menu(g3)](square, square, square)),
                    horizontal(id(g4), horizontal[;gray, cycle_menu(g4)](square)),
                    horizontal(id(g5), horizontal[;gray, menu(g5)](square, square, square)),
                    horizontal(id(g6), horizontal[;gray, cycle_menu(g6)](square, square)),
                    horizontal(id(g7), horizontal[;gray, menu(g7)](square, square, square, square)),
                    horizontal(id(g8), horizontal[;gray, cycle_menu(g8)](square, square, square, square))
                ),
                //           vvvvvvvvvvvvvvv
                column[;blue, menu(tab_blue)](
                    vertical(
                        vertical(select_square, select_square, select_square, select_square),
                        colored_square
                    )
                )
            )
        )
    };
}
