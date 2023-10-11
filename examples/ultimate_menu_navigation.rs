use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use bevy_ui_navigation::prelude::{
    DefaultNavigationPlugins, FocusState, Focusable, NavRequestSystem, NavigationDsl,
};
use bevy_ui_navigation::systems::InputMapping;
use cuicui_chirp::{parse_dsl_impl, ChirpBundle};
use cuicui_layout::{DslBundle, LayoutRootCamera};
use cuicui_layout_bevy_ui::UiDsl;

#[derive(Default, Deref, DerefMut)]
struct UltimateMenuDsl {
    #[deref]
    inner: NavigationDsl<UiDsl>,
}
#[parse_dsl_impl(delegate = inner)]
impl UltimateMenuDsl {}
impl DslBundle for UltimateMenuDsl {
    fn insert(&mut self, cmds: &mut EntityCommands) -> Entity {
        self.inner.insert(cmds)
    }
}

/// THE ULTIMATE MENU DEMONSTRATION
///
/// This is an unrealistic menu demonstrating tabbed navigation, focus memory
/// and navigation hierarchy traversal. It is similar to your classical RPG
/// menu, with the significant difference that **all tabs are shown at the same
/// time on screen** rather than hidden and shown as the tabs are selected.
///
/// We use `cuicui_chirp` for this demo, because using bevy_ui's default spawning
/// mechanism is unrealistic for a complex UI. see the file `assets/ultimate_menu.chirp`
///
/// Use `Q` and `E` to navigate tabs, use `WASD` for moving within containers,
/// `ENTER` and `BACKSPACE` for going down/up the hierarchy.
///
/// Navigation also works with controller
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set({
                let delay = std::time::Duration::from_millis(200);
                AssetPlugin {
                    watch_for_changes: bevy::asset::ChangeWatcher::with_delay(delay),
                    ..default()
                }
            }),
            DefaultNavigationPlugins,
            cuicui_chirp::loader::Plugin::new::<UltimateMenuDsl>(),
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

fn setup(mut commands: Commands, mut input_mapping: ResMut<InputMapping>, serv: Res<AssetServer>) {
    input_mapping.keyboard_navigation = true;
    input_mapping.focus_follows_mouse = true;

    commands.spawn((Camera2dBundle::default(), LayoutRootCamera));

    commands.spawn(ChirpBundle::new(serv.load("ultimate_menu.chirp")));
}
