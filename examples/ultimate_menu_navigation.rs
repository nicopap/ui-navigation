use bevy::prelude::*;

use bevy_ui_build_macros::{build_ui, rect, size, style, unit};
use bevy_ui_navigation::{
    components::FocusableButtonBundle,
    systems::{default_gamepad_input, default_keyboard_input, default_mouse_input, InputMapping},
    FocusState, Focusable, NavMenu, NavigationPlugin,
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
        .init_resource::<InputMapping>()
        .add_plugin(NavigationPlugin)
        .add_startup_system(setup)
        .add_system(button_system)
        .add_system(default_gamepad_input)
        .add_system(default_keyboard_input)
        .add_system(default_mouse_input)
        .run();
}

fn button_system(mut interaction_query: Query<(&Focusable, &mut UiColor), Changed<Focusable>>) {
    for (focus, mut material) in interaction_query.iter_mut() {
        let color = match focus.state() {
            FocusState::Focused => Color::ORANGE_RED,
            FocusState::Active => Color::GOLD,
            FocusState::Dormant => Color::GRAY,
            FocusState::Inert => Color::DARK_GRAY,
        };
        *material = color.into();
    }
}

fn setup(mut commands: Commands) {
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
        ..Default::default()
    });
    let long = FocusableButtonBundle::from(ButtonBundle {
        style: style! {
            size: size!(100 pct, 40 px),
            margin: rect!(2 px),
        },
        ..Default::default()
    });
    let tab_square = FocusableButtonBundle::from(ButtonBundle {
        style: style! {
            size: size!(100 px, 40 px),
            margin: rect!(30 px, 0 px),
        },
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

    let menu = |name| NavMenu::Bound2d.reachable_from_named(name);
    let cycle_menu = |name| NavMenu::Wrapping2d.reachable_from_named(name);
    let named = Name::new;

    // The macro is a very thin wrapper over the "normal" UI declaration
    // technic. Please look at the doc for `build_ui` for info on what it does.
    //
    // Pay attention to calls to `menu("id")`, `cycle_menu("id"), `named`, and
    // `NavMenu::root()`. You'll notice we use `Name` to give a sort of
    // identifier to our focusables so that they are refereable by `NavMenu`s
    // afterward.
    build_ui! {
        #[cmd(commands)]
        // The tab menu should be navigated with `NavRequest::ScopeMove`
        // hence the `WrappingScope`                    vvvvvvvvvvvvvvvvvvvv
        vertical{size:size!(100 pct, 100 pct)}[NavMenu::WrappingScope.root();](
            horizontal{justify_content: FlexStart, flex_basis: unit!(10 pct)}(
                // adding a `Name` component let us refer to those entities
                // later without having to store their `Entity` ids anywhere.
                tab_square[; named("red")],
                tab_square[; named("green")],
                tab_square[; named("blue")]
            ),
            column_box(
                //     vvvvvvvvvvv refers to the "red" `tab_square`
                column[menu("red"); red](
                    vertical(long[; named("select1")], long[; named("select2")]),
                    horizontal{flex_wrap: Wrap}[cycle_menu("select1"); gray](
                        square, square, square, square, square, square, square, square,
                        square, square, square, square, square, square, square, square,
                        square, square, square, square
                    ),
                    horizontal{flex_wrap: Wrap}[cycle_menu("select2"); gray](
                        square, square, square, square, square, square, square, square
                    )
                ),
                //     vvvvvvvvvvvvv refers to the "green" `tab_square`
                column[menu("green"); green](
                    horizontal(long[;named("g1")], horizontal[cycle_menu("g1"); gray](square, square)),
                    horizontal(long[;named("g2")], horizontal[menu("g2");       gray](square)),
                    horizontal(long[;named("g3")], horizontal[cycle_menu("g3"); gray](square, square, square)),
                    horizontal(long[;named("g4")], horizontal[menu("g4");       gray](square, square, square)),
                    horizontal(long[;named("g5")], horizontal[cycle_menu("g5"); gray](square, square)),
                    horizontal(long[;named("g6")], horizontal[menu("g6");       gray](square, square, square)),
                    horizontal(long[;named("g7")], horizontal[cycle_menu("g7"); gray](square, square, square)),
                    horizontal(long[;named("g8")], horizontal[menu("g8");       gray](square, square))
                ),
                //     vvvvvvvvvvvv refers to the "blue" `tab_square`
                column[menu("blue"); blue](
                    vertical(
                        vertical(long, long, long, long),
                        colored_square
                    )
                )
            )
        )
    };
}
