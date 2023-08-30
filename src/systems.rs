//! System for the navigation tree and default input systems to get started.
use crate::{
    events::{Direction, NavRequest, ScopeDirection},
    resolve::{Focusable, Focused},
};

#[cfg(feature = "bevy_ui")]
use crate::resolve::ScreenBoundaries;
use bevy::prelude::*;
#[cfg(feature = "bevy_reflect")]
use bevy::{ecs::reflect::ReflectResource, reflect::Reflect};
#[cfg(feature = "pointer_focus")]
use bevy_mod_picking::prelude::*;

/// Control default ui navigation input buttons
#[derive(Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Resource))]
pub struct InputMapping {
    /// Whether to use keybaord keys for navigation (instead of just actions).
    pub keyboard_navigation: bool,
    /// The gamepads to use for the UI. If empty, default to gamepad 0
    pub gamepads: Vec<Gamepad>,
    /// Deadzone on the gamepad left stick for ui navigation
    pub joystick_ui_deadzone: f32,
    /// X axis of gamepad stick
    pub move_x: GamepadAxisType,
    /// Y axis of gamepad stick
    pub move_y: GamepadAxisType,
    /// Gamepad button for [`Direction::West`] [`NavRequest::Move`]
    pub left_button: GamepadButtonType,
    /// Gamepad button for [`Direction::East`] [`NavRequest::Move`]
    pub right_button: GamepadButtonType,
    /// Gamepad button for [`Direction::North`] [`NavRequest::Move`]
    pub up_button: GamepadButtonType,
    /// Gamepad button for [`Direction::South`] [`NavRequest::Move`]
    pub down_button: GamepadButtonType,
    /// Gamepad button for [`NavRequest::Action`]
    pub action_button: GamepadButtonType,
    /// Gamepad button for [`NavRequest::Cancel`]
    pub cancel_button: GamepadButtonType,
    /// Gamepad button for [`ScopeDirection::Previous`] [`NavRequest::ScopeMove`]
    pub previous_button: GamepadButtonType,
    /// Gamepad button for [`ScopeDirection::Next`] [`NavRequest::ScopeMove`]
    pub next_button: GamepadButtonType,
    /// Gamepad button for [`NavRequest::Unlock`]
    pub free_button: GamepadButtonType,
    /// Keyboard key for [`Direction::West`] [`NavRequest::Move`]
    pub key_left: KeyCode,
    /// Keyboard key for [`Direction::East`] [`NavRequest::Move`]
    pub key_right: KeyCode,
    /// Keyboard key for [`Direction::North`] [`NavRequest::Move`]
    pub key_up: KeyCode,
    /// Keyboard key for [`Direction::South`] [`NavRequest::Move`]
    pub key_down: KeyCode,
    /// Alternative keyboard key for [`Direction::West`] [`NavRequest::Move`]
    pub key_left_alt: KeyCode,
    /// Alternative keyboard key for [`Direction::East`] [`NavRequest::Move`]
    pub key_right_alt: KeyCode,
    /// Alternative keyboard key for [`Direction::North`] [`NavRequest::Move`]
    pub key_up_alt: KeyCode,
    /// Alternative keyboard key for [`Direction::South`] [`NavRequest::Move`]
    pub key_down_alt: KeyCode,
    /// Keyboard key for [`NavRequest::Action`]
    pub key_action: KeyCode,
    /// Keyboard key for [`NavRequest::Cancel`]
    pub key_cancel: KeyCode,
    /// Keyboard key for [`ScopeDirection::Next`] [`NavRequest::ScopeMove`]
    pub key_next: KeyCode,
    /// Alternative keyboard key for [`ScopeDirection::Next`] [`NavRequest::ScopeMove`]
    pub key_next_alt: KeyCode,
    /// Keyboard key for [`ScopeDirection::Previous`] [`NavRequest::ScopeMove`]
    pub key_previous: KeyCode,
    /// Keyboard key for [`NavRequest::Unlock`]
    pub key_free: KeyCode,
    /// Whether mouse hover gives focus to [`Focusable`] elements.
    pub focus_follows_mouse: bool,
}
impl Default for InputMapping {
    fn default() -> Self {
        InputMapping {
            keyboard_navigation: false,
            gamepads: vec![Gamepad { id: 0 }],
            joystick_ui_deadzone: 0.36,
            move_x: GamepadAxisType::LeftStickX,
            move_y: GamepadAxisType::LeftStickY,
            left_button: GamepadButtonType::DPadLeft,
            right_button: GamepadButtonType::DPadRight,
            up_button: GamepadButtonType::DPadUp,
            down_button: GamepadButtonType::DPadDown,
            action_button: GamepadButtonType::South,
            cancel_button: GamepadButtonType::East,
            previous_button: GamepadButtonType::LeftTrigger,
            next_button: GamepadButtonType::RightTrigger,
            free_button: GamepadButtonType::Start,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::W,
            key_down: KeyCode::S,
            key_left_alt: KeyCode::Left,
            key_right_alt: KeyCode::Right,
            key_up_alt: KeyCode::Up,
            key_down_alt: KeyCode::Down,
            key_action: KeyCode::Space,
            key_cancel: KeyCode::Back,
            key_next: KeyCode::E,
            key_next_alt: KeyCode::Tab,
            key_previous: KeyCode::Q,
            key_free: KeyCode::Escape,
            focus_follows_mouse: false,
        }
    }
}

/// `mapping { XYZ::X => ABC::A, XYZ::Y => ABC::B, XYZ::Z => ABC::C }: [(XYZ, ABC)]`
macro_rules! mapping {
    ($($from:expr => $to:expr),* ) => ([$( ( $from, $to ) ),*])
}

/// A system to send gamepad control events to the focus system
///
/// Dpad and left stick for movement, `LT` and `RT` for scopped menus, `A` `B`
/// for selection and cancel.
///
/// The button mapping may be controlled through the [`InputMapping`] resource.
/// You may however need to customize the behavior of this system (typically
/// when integrating in the game) in this case, you should write your own
/// system that sends [`NavRequest`](crate::events::NavRequest) events
pub fn default_gamepad_input(
    mut nav_cmds: EventWriter<NavRequest>,
    has_focused: Query<(), With<Focused>>,
    input_mapping: Res<InputMapping>,
    buttons: Res<Input<GamepadButton>>,
    axis: Res<Axis<GamepadAxis>>,
    mut ui_input_status: Local<bool>,
) {
    use Direction::*;
    use NavRequest::{Action, Cancel, Move, ScopeMove, Unlock};

    if has_focused.is_empty() {
        // Do not compute navigation if there is no focus to change
        return;
    }

    for &gamepad in &input_mapping.gamepads {
        macro_rules! axis_delta {
            ($dir:ident, $axis:ident) => {{
                let axis_type = input_mapping.$axis;
                axis.get(GamepadAxis { gamepad, axis_type })
                    .map_or(Vec2::ZERO, |v| Vec2::$dir * v)
            }};
        }

        let delta = axis_delta!(Y, move_y) + axis_delta!(X, move_x);
        if delta.length_squared() > input_mapping.joystick_ui_deadzone && !*ui_input_status {
            let direction = match () {
                () if delta.y < delta.x && delta.y < -delta.x => South,
                () if delta.y < delta.x => East,
                () if delta.y >= delta.x && delta.y > -delta.x => North,
                () => West,
            };
            nav_cmds.send(Move(direction));
            *ui_input_status = true;
        } else if delta.length_squared() <= input_mapping.joystick_ui_deadzone {
            *ui_input_status = false;
        }

        let command_mapping = mapping! {
            input_mapping.action_button => Action,
            input_mapping.cancel_button => Cancel,
            input_mapping.left_button => Move(Direction::West),
            input_mapping.right_button => Move(Direction::East),
            input_mapping.up_button => Move(Direction::North),
            input_mapping.down_button => Move(Direction::South),
            input_mapping.next_button => ScopeMove(ScopeDirection::Next),
            input_mapping.free_button => Unlock,
            input_mapping.previous_button => ScopeMove(ScopeDirection::Previous)
        };
        for (button_type, request) in command_mapping {
            let button = GamepadButton {
                gamepad,
                button_type,
            };
            if buttons.just_pressed(button) {
                nav_cmds.send(request)
            }
        }
    }
}

/// A system to send keyboard control events to the focus system.
///
/// supports `WASD` and arrow keys for the directions, `E`, `Q` and `Tab` for
/// scopped menus, `Backspace` and `Enter` for cancel and selection.
///
/// The button mapping may be controlled through the [`InputMapping`] resource.
/// You may however need to customize the behavior of this system (typically
/// when integrating in the game) in this case, you should write your own
/// system that sends [`NavRequest`](crate::events::NavRequest) events.
pub fn default_keyboard_input(
    has_focused: Query<(), With<Focused>>,
    keyboard: Res<Input<KeyCode>>,
    input_mapping: Res<InputMapping>,
    mut nav_cmds: EventWriter<NavRequest>,
) {
    use Direction::*;
    use NavRequest::*;

    if has_focused.is_empty() {
        // Do not compute navigation if there is no focus to change
        return;
    }

    let with_movement = mapping! {
        input_mapping.key_up => Move(North),
        input_mapping.key_down => Move(South),
        input_mapping.key_left => Move(West),
        input_mapping.key_right => Move(East),
        input_mapping.key_up_alt => Move(North),
        input_mapping.key_down_alt => Move(South),
        input_mapping.key_left_alt => Move(West),
        input_mapping.key_right_alt => Move(East)
    };
    let without_movement = mapping! {
        input_mapping.key_action => Action,
        input_mapping.key_cancel => Cancel,
        input_mapping.key_next => ScopeMove(ScopeDirection::Next),
        input_mapping.key_next_alt => ScopeMove(ScopeDirection::Next),
        input_mapping.key_free => Unlock,
        input_mapping.key_previous => ScopeMove(ScopeDirection::Previous)
    };
    let mut send_command = |&(key, request)| {
        if keyboard.just_pressed(key) {
            nav_cmds.send(request)
        }
    };
    if input_mapping.keyboard_navigation {
        with_movement.iter().for_each(&mut send_command);
    }
    without_movement.iter().for_each(send_command);
}

/// Update [`ScreenBoundaries`] resource when the UI camera change
/// (assuming there is a unique one).
///
/// See [`ScreenBoundaries`] doc for details.
#[cfg(feature = "bevy_ui")]
#[allow(clippy::type_complexity)]
pub fn update_boundaries(
    mut commands: Commands,
    mut boundaries: Option<ResMut<ScreenBoundaries>>,
    cam: Query<(&Camera, Option<&UiCameraConfig>), Or<(Changed<Camera>, Changed<UiCameraConfig>)>>,
) {
    // TODO: this assumes there is only a single camera with activated UI.
    let first_visible_ui_cam = |(cam, config): (_, Option<&UiCameraConfig>)| {
        config.map_or(true, |c| c.show_ui).then_some(cam)
    };
    let mut update_boundaries = || {
        let cam = cam.iter().find_map(first_visible_ui_cam)?;
        let physical_size = cam.physical_viewport_size()?;
        let new_boundaries = ScreenBoundaries {
            position: Vec2::ZERO,
            screen_edge: crate::resolve::Rect {
                max: physical_size.as_vec2(),
                min: Vec2::ZERO,
            },
            scale: 1.0,
        };
        if let Some(boundaries) = boundaries.as_mut() {
            **boundaries = new_boundaries;
        } else {
            commands.insert_resource(new_boundaries);
        }
        Some(())
    };
    update_boundaries();
}

#[cfg(feature = "pointer_focus")]
fn send_request<E: EntityEvent>(
    f: impl Fn(Query<&Focusable>, Res<ListenerInput<E>>, EventWriter<NavRequest>)
        + Send
        + Sync
        + Copy
        + 'static,
) -> impl Fn() -> On<E> {
    move || On::<E>::run(f)
}

/// Send [`NavRequest`]s when an [`Entity`] is clicked, as defined by
/// [`bevy_mod_picking`].
///
/// # `bevy_mod_picking` features
///
/// `bevy-ui-navigation` inserts the [`DefaultPickingPlugins`].
/// This means you can control how mouse picking works by… picking the
/// feature flags that are most relevant to you:
///
/// Check the [`bevy_mod_picking` feature flags docs.rs page][bmp-features]
/// for a list of features.
///
/// `bevy-ui-navigation` only enables `backed_bevy_ui`, when the `bevy_ui` flag
/// is enabled.
///
/// Depend explicitly on `bevy_mod_picking` and enable the flags you want to
/// extend the picking functionality to, well, 3D objects, sprites, anything
/// really.
///
/// [bmp-features]: https://docs.rs/crate/bevy_mod_picking/0.15.0/features
#[cfg(feature = "pointer_focus")]
#[allow(clippy::type_complexity)]
pub fn enable_click_request(
    input_mapping: Res<InputMapping>,
    to_add: Query<Entity, (With<Focusable>, Without<On<Pointer<Click>>>)>,
    mut commands: Commands,
) {
    use crate::prelude::FocusState::Blocked;

    let on_click = send_request::<Pointer<Click>>(|q, e, mut evs| {
        // TODO(clean): This shouldn't be the responsability of the input system.
        if matches!(q.get(e.listener()), Ok(f) if f.state() != Blocked) {
            evs.send(NavRequest::FocusOn(e.listener()));
            evs.send(NavRequest::Action);
        }
    });
    let on_down = send_request::<Pointer<Down>>(|_, e, mut evs| {
        evs.send(NavRequest::FocusOn(e.listener()));
    });
    let on_over = send_request::<Pointer<Over>>(|_, e, mut evs| {
        evs.send(NavRequest::FocusOn(e.listener()));
    });
    if input_mapping.focus_follows_mouse {
        let cmd_entry = |e| (e, (on_click(), on_down(), on_over()));
        let batch_cmd: Vec<_> = to_add.iter().map(cmd_entry).collect();
        if !batch_cmd.is_empty() {
            commands.insert_or_spawn_batch(batch_cmd);
        }
    } else {
        let cmd_entry = |e| (e, (on_click(), on_down()));
        let batch_cmd: Vec<_> = to_add.iter().map(cmd_entry).collect();
        if !batch_cmd.is_empty() {
            commands.insert_or_spawn_batch(batch_cmd);
        }
    };
}

/// Default input systems for ui navigation.
pub struct DefaultNavigationSystems;
impl Plugin for DefaultNavigationSystems {
    fn build(&self, app: &mut App) {
        use crate::NavRequestSystem;
        app.init_resource::<InputMapping>().add_systems(
            Update,
            (default_gamepad_input, default_keyboard_input).before(NavRequestSystem),
        );

        #[cfg(feature = "bevy_ui")]
        app.add_systems(Update, update_boundaries.before(NavRequestSystem));

        #[cfg(feature = "pointer_focus")]
        app.add_plugins(DefaultPickingPlugins)
            .add_systems(PostUpdate, enable_click_request);
    }
}
