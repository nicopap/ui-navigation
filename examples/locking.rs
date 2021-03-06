use bevy::prelude::*;

use bevy_ui_navigation::{
    DefaultNavigationPlugins, FocusState, Focusable, NavEvent, NavRequestSystem,
};

/// This example illustrates how to make a button "lock". To lock the UI, press
/// 'A' on controller or 'left click' on mouse when the button with the lock is
/// focused.
///
/// To leave lock mode, press 'escape' on keyboard or 'start' on controller.
/// This will emit a `NavRequest::Free` in the default input systems. Allowing
/// the focus to change again.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultNavigationPlugins)
        .init_resource::<Images>()
        .add_startup_system(setup)
        .add_system(button_system.after(NavRequestSystem))
        .add_system(print_nav_events.after(NavRequestSystem))
        .run();
}

fn print_nav_events(mut events: EventReader<NavEvent>) {
    for event in events.iter() {
        println!("{:?}", event);
    }
}

#[allow(clippy::type_complexity)]
fn button_system(
    mut interaction_query: Query<(&Focusable, &mut UiColor), (Changed<Focusable>, With<Button>)>,
) {
    for (focus, mut material) in interaction_query.iter_mut() {
        if let FocusState::Focused = focus.state() {
            *material = Color::ORANGE_RED.into();
        } else {
            *material = Color::DARK_GRAY.into();
        }
    }
}

struct Images {
    lock: UiImage,
}
impl FromWorld for Images {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        Images {
            lock: assets.load("lock.png").into(),
        }
    }
}

fn setup(mut commands: Commands, imgs: Res<Images>) {
    let center_pct = |v: usize| Val::Percent((v as f32) * 25.0 + 25.0);
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for x in 0..3 {
                for y in 0..3 {
                    let position = Rect {
                        left: center_pct(x),
                        bottom: center_pct(y),
                        ..Default::default()
                    };
                    let bundle = button_bundle(position);
                    let mut button_cmds = commands.spawn_bundle(bundle);
                    if x == 1 && y == 1 {
                        // We set the center button as "lock", pressing Action
                        // while it is focused will block the navigation system
                        //                 vvvvvvvvvvvvvvvvv
                        button_cmds.insert(Focusable::lock()).with_children(|cmds| {
                            cmds.spawn_bundle(ImageBundle {
                                image: imgs.lock.clone(),
                                ..Default::default()
                            });
                        });
                    } else {
                        button_cmds.insert(Focusable::default());
                    }
                }
            }
        });
}
fn button_bundle(position: Rect<Val>) -> ButtonBundle {
    ButtonBundle {
        style: Style {
            size: Size::new(Val::Px(95.0), Val::Px(65.0)),
            position,
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        color: Color::DARK_GRAY.into(),
        ..Default::default()
    }
}
