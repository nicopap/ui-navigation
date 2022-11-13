//! System for the navigation tree and default input systems to get started.
use crate::{
    events::{Direction, NavRequest, ScopeDirection},
    resolve::{FocusState, Focusable, Focused, ScreenBoundaries},
};

use bevy::utils::FloatOrd;
use bevy::{ecs::system::SystemParam, prelude::*};

/// Control default ui navigation input buttons
#[derive(Resource)]
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
    /// Mouse button for [`NavRequest::Action`]
    pub mouse_action: MouseButton,
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
            mouse_action: MouseButton::Left,
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

/// [`SystemParam`](https://docs.rs/bevy/0.7.0/bevy/ecs/system/trait.SystemParam.html)
/// used to compute UI focusable physical positions in mouse input systems.
#[derive(SystemParam)]
pub struct NodePosQuery<'w, 's, T: Component> {
    entities: Query<
        'w,
        's,
        (
            Entity,
            &'static T,
            &'static GlobalTransform,
            &'static Focusable,
        ),
    >,
    boundaries: Option<Res<'w, ScreenBoundaries>>,
}
impl<'w, 's, T: Component> NodePosQuery<'w, 's, T> {
    fn cursor_pos(&self, at: Vec2) -> Option<Vec2> {
        let boundaries = self.boundaries.as_ref()?;
        Some(at * boundaries.scale + boundaries.position)
    }
}

fn is_in_node<T: ScreenSize>(
    at: Vec2,
    (_, node, trans, _): &(Entity, &T, &GlobalTransform, &Focusable),
) -> bool {
    let ui_pos = trans.translation().truncate();
    let node_half_size = node.size() / 2.0;
    let min = ui_pos - node_half_size;
    let max = ui_pos + node_half_size;
    (min.x..max.x).contains(&at.x) && (min.y..max.y).contains(&at.y)
}

/// Check which [`Focusable`] is at position `at` if any.
///
/// NOTE: returns `None` if there is no [`ScreenBoundaries`] resource.
pub fn ui_focusable_at<T>(at: Vec2, query: &NodePosQuery<T>) -> Option<Entity>
where
    T: ScreenSize + Component,
{
    let world_at = query.cursor_pos(at)?;
    query
        .entities
        .iter()
        .filter(|query_elem| is_in_node(world_at, query_elem))
        .max_by_key(|elem| FloatOrd(elem.2.translation().z))
        .map(|elem| elem.0)
}

fn cursor_pos(windows: &Windows) -> Option<Vec2> {
    windows.get_primary().and_then(|w| w.cursor_position())
}

pub trait ScreenSize {
    fn size(&self) -> Vec2;
}

#[cfg(feature = "bevy_ui")]
impl ScreenSize for Node {
    fn size(&self) -> Vec2 {
        self.size()
    }
}

/// A system to send mouse control events to the focus system
///
/// Unlike [`generic_default_mouse_input`], this system is gated by the
/// `bevy_ui` feature. It relies on bevy/render specific types:
/// `bevy::render::Camera` and `bevy::ui::Node`.
///
/// Which button to press to cause an action event is specified in the
/// [`InputMapping`] resource.
///
/// You may however need to customize the behavior of this system (typically
/// when integrating in the game) in this case, you should write your own
/// system that sends [`NavRequest`](crate::events::NavRequest) events. You may use
/// [`ui_focusable_at`] to tell which focusable is currently being hovered.
#[cfg(feature = "bevy_ui")]
#[allow(clippy::too_many_arguments)]
pub fn default_mouse_input(
    input_mapping: Res<InputMapping>,
    windows: Res<Windows>,
    mouse: Res<Input<MouseButton>>,
    focusables: NodePosQuery<Node>,
    focused: Query<Entity, With<Focused>>,
    nav_cmds: EventWriter<NavRequest>,
    last_pos: Local<Vec2>,
) {
    generic_default_mouse_input(
        input_mapping,
        windows,
        mouse,
        focusables,
        focused,
        nav_cmds,
        last_pos,
    );
}

/// A generic system to send mouse control events to the focus system
///
/// `T` must be a component assigned to `Focusable` elements that implements
/// the [`ScreenSize`] trait.
///
/// Which button to press to cause an action event is specified in the
/// [`InputMapping`] resource.
///
/// You may however need to customize the behavior of this system (typically
/// when integrating in the game) in this case, you should write your own
/// system that sends [`NavRequest`](crate::events::NavRequest) events. You may use
/// [`ui_focusable_at`] to tell which focusable is currently being hovered.
#[allow(clippy::too_many_arguments)]
pub fn generic_default_mouse_input<T: ScreenSize + Component>(
    input_mapping: Res<InputMapping>,
    windows: Res<Windows>,
    mouse: Res<Input<MouseButton>>,
    focusables: NodePosQuery<T>,
    focused: Query<Entity, With<Focused>>,
    mut nav_cmds: EventWriter<NavRequest>,
    mut last_pos: Local<Vec2>,
) {
    let no_focusable_msg = "Entity with `Focused` component must also have a `Focusable` component";
    let cursor_pos = match cursor_pos(&windows) {
        Some(c) => c,
        None => return,
    };
    let world_cursor_pos = match focusables.cursor_pos(cursor_pos) {
        Some(c) => c,
        None => return,
    };
    let released = mouse.just_released(input_mapping.mouse_action);
    let pressed = mouse.pressed(input_mapping.mouse_action);
    let focused = focused.get_single();

    // Return early if cursor didn't move since last call
    let camera_moved = focusables.boundaries.map_or(false, |b| b.is_changed());
    let mouse_moved = *last_pos != cursor_pos;
    if (!released && !pressed) && !mouse_moved && !camera_moved {
        return;
    } else {
        *last_pos = cursor_pos;
    }
    // we didn't do it earlier so that we can leave early when the camera didn't move
    let pressed = input_mapping.focus_follows_mouse || pressed;

    let hovering_focused = |focused| {
        let focused = focusables.entities.get(focused).expect(no_focusable_msg);
        is_in_node(world_cursor_pos, &focused)
    };
    // If the currently hovered node is the focused one, there is no need to
    // find which node we are hovering and to switch focus to it (since we are
    // already focused on it)
    let hovering = focused.map_or(false, hovering_focused);
    let set_focused = (pressed || released) && !hovering;
    if set_focused {
        // We only run this code when we really need it because we iterate over all
        // focusables, which can eat a lot of CPU.
        let under_mouse = focusables
            .entities
            .iter()
            .filter(|query_elem| query_elem.3.state() != FocusState::Blocked)
            .filter(|query_elem| is_in_node(world_cursor_pos, query_elem))
            .max_by_key(|elem| FloatOrd(elem.2.translation().z))
            .map(|elem| elem.0);
        let to_target = match under_mouse {
            Some(c) => c,
            None => return,
        };
        nav_cmds.send(NavRequest::FocusOn(to_target));
    }
    if released && (set_focused || hovering) {
        nav_cmds.send(NavRequest::Action);
    }
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
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
) {
    // TODO: this assumes there is only a single camera with activated UI.
    let first_visible_ui_cam = |(cam, config): (_, Option<&UiCameraConfig>)| {
        config.map_or(true, |c| c.show_ui).then_some(cam)
    };
    let mut update_boundaries = || {
        let cam = cam.iter().find_map(first_visible_ui_cam)?;
        let target_info = cam.target.get_render_target_info(&windows, &images)?;
        let new_boundaries = ScreenBoundaries {
            position: Vec2::ZERO,
            screen_edge: crate::resolve::Rect {
                max: target_info.physical_size.as_vec2(),
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

/// Default input systems for ui navigation.
#[cfg(feature = "bevy_ui")]
pub struct DefaultNavigationSystems;
#[cfg(feature = "bevy_ui")]
impl Plugin for DefaultNavigationSystems {
    fn build(&self, app: &mut App) {
        use crate::NavRequestSystem;
        app.init_resource::<InputMapping>()
            .add_system(default_mouse_input.before(NavRequestSystem))
            .add_system(default_gamepad_input.before(NavRequestSystem))
            .add_system(default_keyboard_input.before(NavRequestSystem))
            .add_system(
                update_boundaries
                    .before(NavRequestSystem)
                    .before(default_mouse_input),
            );
    }
}
