use bevy::input::{keyboard::KeyboardInput, ElementState};
use bevy::prelude::*;

use bevy_ui_navigation::{Direction, Focusable, NavEvent, NavMenu, NavRequest, NavigationPlugin};

/// This example demonstrates a more complex menu system where you navigate
/// through menus and go to submenus using the `Action` and `Cancel`
/// (`ENTER` and `BACKSPACE` on keyboard) requests.
///
/// This introduces the concept of "active" and "dormant" focusable elements.
///
/// The focus goes back to active elements from the parent menu if you request
/// `Cancel` in a given submenu.
///
/// The focus goes back to the child menu's dormant element if you request
/// `Action` while the parent menu's corresponding `Focusable` is focused.
///
/// To navigate to the right column, move focus to the button with the right arrow
/// and press `ENTER`, to navigate to the left, press `BACKSPACE`. Notice how
/// going back to an already explored menu sets the focused element to the last
/// focused one.
///
/// This example also demonstrates the `NavRequest::FocusOn` request. When
/// `ENTER` is pressed when a green circle button is focused, it sends the
/// `FocusOn` request with a first row button as target.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin)
        .init_resource::<Materials>()
        .insert_resource(Gameui::new())
        .add_startup_system(setup)
        .add_system(query_bad_stuff)
        .add_system(button_system)
        .add_system(keyboard_input)
        .add_system(handle_nav_events)
        .run();
}

struct Gameui {
    from: Vec<Entity>,
    to: Entity,
}
impl Gameui {
    pub fn new() -> Self {
        Self {
            from: Vec::new(),
            to: Entity::new(1),
        }
    }
}

fn query_bad_stuff(query: Query<(Entity, &NavMenu, &Focusable), Changed<Focusable>>) {
    if !query.is_empty() {
        println!("BAD STUFF: {:?}", query.iter().collect::<Vec<_>>());
    }
}

struct Materials {
    inert: Handle<ColorMaterial>,
    focused: Handle<ColorMaterial>,
    active: Handle<ColorMaterial>,
    dormant: Handle<ColorMaterial>,
    background: Handle<ColorMaterial>,
    rarrow: Handle<ColorMaterial>,
    circle: Handle<ColorMaterial>,
}

impl FromWorld for Materials {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        let rarrow = assets.load("rarrow.png").into();
        let circle = assets.load("green_circle.png").into();
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        Materials {
            inert: materials.add(Color::DARK_GRAY.into()),
            focused: materials.add(Color::ORANGE_RED.into()),
            active: materials.add(Color::GOLD.into()),
            dormant: materials.add(Color::GRAY.into()),
            background: materials.add(Color::BLACK.into()),
            rarrow: materials.add(rarrow),
            circle: materials.add(circle),
        }
    }
}

fn keyboard_input(mut keyboard: EventReader<KeyboardInput>, mut nav_cmds: EventWriter<NavRequest>) {
    use Direction::*;
    use NavRequest::*;
    let command_mapping = |code| match code {
        KeyCode::Return => Some(Action),
        KeyCode::Back => Some(Cancel),
        KeyCode::Up => Some(Move(North)),
        KeyCode::Down => Some(Move(South)),
        KeyCode::Left => Some(Move(West)),
        KeyCode::Right => Some(Move(East)),
        _ => None,
    };
    for event in keyboard.iter() {
        if event.state == ElementState::Released {
            if let Some(cmd) = event.key_code.and_then(command_mapping) {
                nav_cmds.send(cmd)
            }
        }
    }
}

fn button_system(
    materials: Res<Materials>,
    mut interaction_query: Query<
        (&Focusable, &mut Handle<ColorMaterial>),
        (Changed<Focusable>, With<Button>),
    >,
) {
    for (focus_state, mut material) in interaction_query.iter_mut() {
        println!("REDRAW {:?}", focus_state);
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

fn handle_nav_events(
    mut events: EventReader<NavEvent>,
    mut requests: EventWriter<NavRequest>,
    game: Res<Gameui>,
) {
    use NavRequest::Action;
    for event in events.iter() {
        println!("{:?}", event);
        match event {
            NavEvent::NoChanges {
                from,
                request: Action,
            } if game.from.contains(from.first()) => requests.send(NavRequest::FocusOn(game.to)),
            _ => {}
        }
    }
}

fn menu(materials: &Materials) -> NodeBundle {
    let size_fn = |width, height| Size::new(Val::Percent(width), Val::Percent(height));
    let size = size_fn(20.0, 95.0);
    let style = Style {
        size,
        flex_direction: FlexDirection::Column,
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::Center,
        align_content: AlignContent::Stretch,
        ..Default::default()
    };
    let material = materials.background.clone();
    NodeBundle {
        style,
        material,
        ..Default::default()
    }
}
fn setup(mut commands: Commands, materials: Res<Materials>, mut game: ResMut<Gameui>) {
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());

    let size_fn = |width, height| Size::new(Val::Percent(width), Val::Percent(height));
    let style = Style {
        position_type: PositionType::Absolute,
        flex_direction: FlexDirection::Row,
        size: size_fn(100.0, 100.0),
        ..Default::default()
    };
    let bundle = NodeBundle {
        style,
        ..Default::default()
    };
    let image_style = Style {
        size: size_fn(100.0, 100.0),
        ..Default::default()
    };
    let rarrow = ImageBundle {
        style: image_style.clone(),
        material: materials.rarrow.clone(),
        ..Default::default()
    };
    let circle = ImageBundle {
        style: image_style,
        material: materials.circle.clone(),
        ..Default::default()
    };

    commands
        .spawn_bundle(bundle)
        .insert(NavMenu::root())
        .with_children(|commands| {
            let mut next_menu_button: Option<Entity> = None;
            for j in 0..5 {
                commands
                    .spawn_bundle(menu(&materials))
                    .insert(NavMenu::new(next_menu_button).cycling())
                    .with_children(|commands| {
                        for i in 0..4 {
                            let mut button = commands.spawn_bundle(button(&materials));
                            button.insert(Focusable::default());
                            if j == 0 && i == 3 {
                                game.to = button.id();
                            }
                            if j == i {
                                button.with_children(|commands| {
                                    commands.spawn_bundle(rarrow.clone());
                                });
                                next_menu_button = Some(button.id());
                            }
                            if j == 4 {
                                let to_add = button
                                    .with_children(|commands| {
                                        commands.spawn_bundle(circle.clone());
                                    })
                                    .id();
                                game.from.push(to_add);
                            }
                        }
                    });
            }
        });
}
fn button(materials: &Materials) -> ButtonBundle {
    let size_fn = |width, height| Size::new(Val::Percent(width), Val::Percent(height));
    let size = size_fn(95.0, 12.0);
    let style = Style {
        size,
        margin: Rect::all(Val::Percent(3.0)),
        ..Default::default()
    };
    let material = materials.inert.clone();
    ButtonBundle {
        style,
        material,
        ..Default::default()
    }
}
