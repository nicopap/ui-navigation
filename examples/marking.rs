use bevy::prelude::*;

use bevy_ui_navigation::{
    DefaultNavigationPlugins, FocusState, Focusable, Focused, NavMarkerPropagationPlugin, NavMenu,
    NavRequestSystem,
};

macro_rules! column_type {
    (enum $type_name:ident , $i_base:expr) => {
        #[derive(Component, Clone, Debug)]
        enum $type_name {
            Top,
            Middle,
            Bottom,
        }
        impl $type_name {
            fn i(&self) -> usize {
                match *self {
                    $type_name::Top => $i_base + 0,
                    $type_name::Middle => $i_base + 1,
                    $type_name::Bottom => $i_base + 2,
                }
            }
        }
    };
}
column_type!(enum LeftColMenu, 0);
column_type!(enum CenterColMenu, 3);
column_type!(enum RightColMenu, 6);

/// This example demonstrates the `marker` module features.
///
/// It demonstrates:
/// 1. How to register multiple marking types
/// 2. How to add menu markers that automatically add components to focusables
///    within the menu
/// 3. How to use the marker components to tell menus involved in `NavEvent`
///    events.
///
/// It constructs 9 menus of 3 buttons, you can navigate between them with the
/// leftmost/rightmost buttons in the menus.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // We must add the NavMarker plugin for each menu marker types we want
        .add_plugin(NavMarkerPropagationPlugin::<LeftColMenu>::new())
        .add_plugin(NavMarkerPropagationPlugin::<CenterColMenu>::new())
        .add_plugin(NavMarkerPropagationPlugin::<RightColMenu>::new())
        .add_plugins(DefaultNavigationPlugins)
        .add_startup_system(setup)
        .add_system(button_system.after(NavRequestSystem))
        .add_system(print_menus.after(NavRequestSystem))
        .run();
}

fn print_menus(
    left_menus: Query<&LeftColMenu, Added<Focused>>,
    center_menus: Query<&CenterColMenu, Added<Focused>>,
    right_menus: Query<&RightColMenu, Added<Focused>>,
) {
    // To do something when entering a menu, you use a `Query` on a
    // component specified in the `NavMarkerPropagationPlugin`
    //
    // Notice in `setup` how we DID NOT add any `*ColumnMenus` components to
    // any entity? It is the `NavMarkerPropagationPlugin` that added the
    // components to the focusables within the `MarkingMenu`.
    if let Ok(menu) = left_menus.get_single() {
        println!("Entered Red column menu: {menu:?}");
    }
    if let Ok(menu) = center_menus.get_single() {
        println!("Entered Green column menu: {menu:?}");
    }
    if let Ok(menu) = right_menus.get_single() {
        println!("Entered Blue column menu: {menu:?}");
    }
}

fn button_system(mut interaction_query: Query<(&Focusable, &mut UiColor), Changed<Focusable>>) {
    for (focus, mut material) in interaction_query.iter_mut() {
        let color = match focus.state() {
            FocusState::Focused => Color::ORANGE,
            FocusState::Active => Color::GOLD,
            FocusState::Dormant => Color::GRAY,
            FocusState::Inert => Color::BLACK,
        };
        *material = color.into();
    }
}

fn setup(mut commands: Commands) {
    use FlexDirection::{ColumnReverse, Row};
    use NavMenu::Wrapping2d as WrapMenu;
    use Val::{Percent as Pct, Px};
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());

    // First argument to `bndl!` is the color of the node, second is the Style
    macro_rules! bndl {
        ($color:expr, {$($style:tt)*} ) => (
            NodeBundle {
                color: ($color as Color).into(),
                style: Style {
                    $($style)*
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    }
    let good_margin = Rect::all(Val::Px(20.0));
    // white background
    let root = bndl!(Color::WHITE, {
        size: Size::new(Pct(100.0), Pct(100.0)),
        flex_direction: Row,
    });
    // root menu to access each `cell`
    let keyboard = bndl!(Color::DARK_GRAY, {
        size: Size::new(Px(50.0 * 3.2), Px(50.0 * 3.2)),
        flex_direction: ColumnReverse,
        flex_wrap: FlexWrap::Wrap,
    });
    // black container
    let billboard = bndl!(Color::BLACK, { flex_direction: Row, margin: good_margin, });
    // colored columns
    let column = |color| bndl!(color, { flex_direction: ColumnReverse, margin: good_margin, });
    // each row of a column
    let cell = bndl!(Color::rgba(1.0, 1.0, 1.0, 0.2), {
        flex_direction: Row,
        margin: good_margin,
        padding: good_margin,
    });
    // navigable buttons within columns
    let button = bndl!(Color::BLACK, {
        size: Size::new(Px(40.0), Px(40.0)),
        margin: Rect::all(Px(5.0)),
    });
    // spawn nine different buttons for the keyboard menu
    macro_rules! nine {
        ($k:expr) => {
            [$k, $k, $k, $k, $k, $k, $k, $k, $k]
        };
    }
    let bts: [Entity; 9] = nine![commands
        .spawn_bundle(button.clone())
        .insert(Focusable::default())
        .id()];
    // create a cell in a column, with three navigable buttons
    macro_rules! spawn_cell {
        ($cmds: expr) => {{
            $cmds.spawn_bundle(cell.clone()).with_children(|cmds| {
                let focus = || Focusable::default();
                cmds.spawn_bundle(button.clone()).insert(focus());
                cmds.spawn_bundle(button.clone()).insert(focus());
                cmds.spawn_bundle(button.clone()).insert(focus());
            })
        }};
    }
    let (red, green, blue) = (Color::RED, Color::GREEN, Color::BLUE);
    // spawn the whole UI tree
    commands.spawn_bundle(root).with_children(|cmds| {
        cmds.spawn_bundle(keyboard)
            // Add root menu        vvvvvvvvvvvvvvvvvvvv
            .insert_bundle(NavMenu::WrappingScope.root())
            .push_children(&bts);

        cmds.spawn_bundle(billboard).with_children(|cmds| {
            // Note: each colored column has a different type, but
            // within each column there are three menus (Top, Middle, Bottom)
            //
            // in `print_menus`, we detect the menu in which we are
            // using the `Query<&LeftColumnMenus>` query.
            //
            // `WrapMenu` = `NavMenu::Wrapping2d`, see type alias on top of this
            // function.
            cmds.spawn_bundle(column(red)).with_children(|cmds| {
                let menu = |row: LeftColMenu| WrapMenu.reachable_from(bts[row.i()]).marking(row);
                spawn_cell!(cmds).insert_bundle(menu(LeftColMenu::Top));
                spawn_cell!(cmds).insert_bundle(menu(LeftColMenu::Middle));
                spawn_cell!(cmds).insert_bundle(menu(LeftColMenu::Bottom));
            });
            cmds.spawn_bundle(column(green)).with_children(|cmds| {
                let menu = |row: CenterColMenu| WrapMenu.reachable_from(bts[row.i()]).marking(row);
                spawn_cell!(cmds).insert_bundle(menu(CenterColMenu::Top));
                spawn_cell!(cmds).insert_bundle(menu(CenterColMenu::Middle));
                spawn_cell!(cmds).insert_bundle(menu(CenterColMenu::Bottom));
            });
            cmds.spawn_bundle(column(blue)).with_children(|cmds| {
                let menu = |row: RightColMenu| WrapMenu.reachable_from(bts[row.i()]).marking(row);
                spawn_cell!(cmds).insert_bundle(menu(RightColMenu::Top));
                spawn_cell!(cmds).insert_bundle(menu(RightColMenu::Middle));
                spawn_cell!(cmds).insert_bundle(menu(RightColMenu::Bottom));
            });
        });
    });
}
