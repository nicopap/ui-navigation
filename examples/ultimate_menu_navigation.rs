use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use bevy_ui_navigation::prelude::{
    DefaultNavigationPlugins, FocusState, Focusable, NavRequestSystem, NavigationDsl,
};
use bevy_ui_navigation::systems::InputMapping;
use cuicui_layout::{dsl, dsl_functions::*, DslBundle, LayoutRootCamera, LeafRule, Size};
use cuicui_layout_bevy_ui::UiDsl;

#[derive(Clone, Copy, Debug)]
enum ButtonStyle {
    Long,
    Medium,
    Square,
}

#[derive(Default, Deref, DerefMut)]
struct UltimateMenuDsl(NavigationDsl<UiDsl>);
impl UltimateMenuDsl {
    fn button(&mut self, style: ButtonStyle, cmds: &mut EntityCommands) -> Entity {
        cmds.insert((
            Focusable::default(),
            cuicui_layout::Node::Box(Size {
                width: match style {
                    ButtonStyle::Long => LeafRule::Parent(0.8),
                    ButtonStyle::Medium => LeafRule::Fixed(90.0),
                    ButtonStyle::Square => LeafRule::Fixed(30.0),
                },
                height: LeafRule::Fixed(30.0),
            }),
        ))
        .id()
    }
}
impl DslBundle for UltimateMenuDsl {
    fn insert(&mut self, cmds: &mut EntityCommands) -> Entity {
        self.0.insert(cmds)
    }
}
type Dsl = UltimateMenuDsl;

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
        .add_plugins((
            DefaultPlugins,
            DefaultNavigationPlugins,
            cuicui_layout_bevy_ui::Plugin,
        ))
        .add_systems(Startup, setup)
        // IMPORTANT: setting the button appearance update system after the
        // NavRequestSystem makes everything much snappier, highly recommended.
        .add_systems(
            Update,
            (
                block_some_focusables.before(NavRequestSystem),
                button_system.after(NavRequestSystem),
            ),
        )
        .run();
}

fn block_some_focusables(
    mut focusables: Query<&mut Focusable>,
    mut blocked_index: Local<usize>,
    time: Res<Time>,
) {
    let delta = time.delta_seconds_f64();
    let current_time = time.elapsed_seconds_f64();
    let at_interval = |t: f64| current_time % t < delta;

    if at_interval(3.0) {
        let mut skipped = focusables.iter_mut().skip(*blocked_index);
        if skipped.len() == 0 {
            *blocked_index = 0;
        }
        *blocked_index += 3;
        for mut to_unblock in skipped.by_ref().take(3) {
            to_unblock.unblock();
        }
        for mut to_block in skipped.take(3) {
            to_block.block();
        }
    }
}

fn button_system(
    mut interaction_query: Query<(&Focusable, &mut BackgroundColor), Changed<Focusable>>,
) {
    for (focus, mut material) in interaction_query.iter_mut() {
        let color = match focus.state() {
            FocusState::Focused => Color::ORANGE_RED,
            FocusState::Active => Color::GOLD,
            FocusState::Prioritized => Color::GRAY,
            FocusState::Inert => Color::DARK_GRAY,
            FocusState::Blocked => Color::ANTIQUE_WHITE,
        };
        *material = color.into();
    }
}

fn grid(
    width: u32,
    height: u32,
    cmds: &mut ChildBuilder,
    mut spawner: impl FnMut(&mut ChildBuilder, u32, u32),
) {
    for x in 0..width {
        let entity = cmds.spawn_empty();
        dsl!(entity,
            column(named format!("col{x}"), rules(child(1.05), pct(97))) {
                code(let cmds) {
                    (0..height).for_each(|y| spawner(cmds, x, y))
                }
            }
        )
    }
}

fn setup(mut commands: Commands, mut input_mapping: ResMut<InputMapping>) {
    use ButtonStyle::{Long, Medium, Square};

    input_mapping.keyboard_navigation = true;
    input_mapping.focus_follows_mouse = true;

    // ui camera
    commands.spawn((Camera2dBundle::default(), LayoutRootCamera));

    let (red, green, blue) = (Color::RED, Color::GREEN, Color::BLUE);
    let semi = Color::rgba(0.9, 0.9, 0.9, 0.3);

    // This uses `cuicui_dsl`'s macro.
    // check the documentation at: <https://docs.rs/cuicui_dsl/latest/cuicui_dsl/macro.dsl.html>
    //
    // Pay attention to the `menu "name"`
    let green_row = |cmds: &mut ChildBuilder, name: String, percent| {
        dsl! { cmds,
        row(named format!("{name}_menu"), width pct(99)) {
            spawn(focus, rules(pct(percent), px(30)), named name.clone());
            row(named format!("{name}_sub"), menu name, bg semi, rules(pct(98 - percent), child(1.05)), main_margin 3.) {
                code(let cmds) {
                    let count = (98 - percent) / 10;
                    (0..count).for_each(|i|
                        dsl!(cmds, button(Square, named i.to_string());)
                    );
                }
            }
        }
        }
    };
    let columns = |cmds: &mut ChildBuilder| {
        dsl! { cmds,
        column(menu "red", "red_menu", bg red, rules(pct(32), pct(97)), main_margin 10.) {
            button(Long, "red1");
            button(Long, "red2");
            row(menu "red1", wrap, "red1_menu", bg semi, rules(pct(75), pct(30))) {
                code(let cmds) {
                    grid(5, 3, cmds, |cmds, x, y|
                        dsl!(cmds, button(Square, named format!("{x}×{y}"));)
                    )
                }
            }
            row(menu "red2", "red2_menu", bg semi, main_margin 5., rules(pct(90), pct(50))) {
                code(let cmds) {
                    grid(10, 5, cmds, |cmds, x, y|
                        dsl!(cmds, button(Square, named format!("{x}×{y}"));)
                    )
                }
            }
        }
        column(menu "green", bg green, "green_menu", rules(pct(32), pct(97))) {
            code(let cmds) {
                for (i, pct)  in [85, 70, 55, 40, 25, 5, 85, 55].iter().enumerate() {
                    let name = format!("green_{i}");
                    green_row(cmds, name, *pct);
                }
            }
        }
        column(menu "blue", bg blue, "blue_menu", rules(pct(32), pct(97))) {
            button(Long, "blue1");
            button(Long, "blue2");
            button(Long, "blue3");
            button(Long, "blue4");
        }
        }
    };
    dsl! {&mut commands,
        column(screen_root, distrib_end, "Root Root") {
            // The tab menu should be navigated with `NavRequest::ScopeMove` hence the `scope`
            row("tab menu", wrap, menu_root, scope, align_end, rules(pct(50), child(1.05)), main_margin 20.) {
                // adding a `Name` component (this is what `named "red"` does)
                // let us refer to those entities later without having to store their
                // `Entity` ids anywhere.
                button(named "red", border(5, red));
                button(Medium, named "green", border(5, green));
                button(Medium, named "blue", border(5, blue));
            }
            row("columns container", align_start, rules(pct(99), pct(80))) {
                code(let cmds) {
                    columns(cmds);
                }
            }
        }
    };
}
