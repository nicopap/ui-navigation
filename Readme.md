# Bevy UI navigation

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![Latest version](https://img.shields.io/crates/v/bevy_ui_navigation.svg)](https://crates.io/crates/bevy_ui_navigation)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)
[![Documentation](https://docs.rs/bevy-ui-navigation/badge.svg)](https://docs.rs/bevy-ui-navigation/)

A generic UI navigation algorithm meant to be adaptable to any UI library, but
currently limiting itself to targeting the Bevy engine default UI library.

```toml
[dependencies]
bevy-ui-navigation = "0.9.0"
```

The in-depth design specification is [available here](https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md).

### Examples

Check out the `examples` directory for bevy examples.

Ultimate navigation example demo
![Demonstration of "Ultimate navigation"
example](https://user-images.githubusercontent.com/26321040/141612751-ba0e62b2-23d6-429a-b5d1-48b09c10d526.gif)

## Usage

See [this example](https://github.com/nicopap/ui-navigation/blob/master/examples/flat_2d_nav.rs)
for a quick start guide.

### Simple case

You just have a collection of buttons displayed on screen and you want the
player to be able to select between them with their controller? Simply use the
`bevy_ui_navigation::components::FocusableButtonBundle` instead of
`ButtonBundle`. The navigation system is capable of switching focus based on
the 2D position of each focusable element.

This won't work out of the box though, you must wire the UI to the control
scheme, by sending requests to the `NavRequest` event queue. We do not provide
out of the box a way to do this, but we provide default input handling systems.
Try this:
```rust
use bevy_ui_navigation::systems::{default_gamepad_input, default_keyboard_input, InputMapping};
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin)
        .init_resource::<InputMapping>()
        .add_system(default_keyboard_input)
        .add_system(default_gamepad_input)
        .run();
}
```
The default button mapping may not be what you want, or you may want to change it
in-game (for example when the user is in an input mapping menu) The controls
are modifiable with the `InputMapping` resource. Check out the doc for it for
more details.

Check the `examples` directory for more example code.

To respond to relevant user input, for example when the player pressed the
"Action" button when focusing `start_game_button`, you should read the
`NavEvent` event queue:
```rust
fn handle_nav_events(mut events: EventReader<NavEvent>, game: Res<Gameui>) {
    use bevy_ui_navigation::{NavEvent::NoChanges, NavRequest::Action};
    for event in events.iter() {
        match event {
            NoChanges { from, request: Action } if from.first() == game.start_game_button => {
                  // Start the game on "A" or "ENTER" button press
            }
            _ => {}
        }
    }
}
```
The focus navigation works across the whole UI tree, regardless of how or where
you've put your focusable entities. You just move in the direction you want to
go, and you get there.

In truth, `FocusableButtonBundle` is just a `ButtonBundle` with an extra
`Focusable` component field.

Any `Entity` can be converted into a focusable entity by adding the `Focusable`
component to it. To do so, just:
```rust
commands.entity(my_entity).insert(Focusable::default());
```
That's it! Now `my_entity` is part of the navigation tree. The player can
select it with their controller the same way as any other `Focusable` element.

You probably want to render the focused button differently than other buttons,
this can be done with the `Changed<Focusable>` query parameter as follow:
```rust
fn button_system(
    materials: Res<Materials>,
    mut focusables: Query<(&Focusable, &mut Handle<ColorMaterial>), Changed<Focusable>>,
) {
    for (focus_state, mut material) in focusables.iter_mut() {
        if focus_state.is_focused() {
            *material = materials.focused.clone();
        } else {
            *material = materials.inert.clone();
        }
    }
}
```

### More complex use cases

### With `NavMenu`s

Suppose you have a more complex game with menus sub-menus and sub-sub-menus etc.
For example, in your everyday 2021 AAA game, to change the antialiasing you
would go through a few menus:
```
game menu → options menu → graphics menu → custom graphics menu → AA
```
In this case, you need to be capable of specifying which button in the previous
menu leads to the next menu (for example, you would press the "Options" button
in the game menu to access the options menu).

For that, you need to use the `NavMenu` component.
1. First you need a "root" `NavMenu`. This is a limit of the navigation
   algorithm (which may be fixed in the future)
2. You also need the `Entity` value of your "Options" button
3. You then add a `NavMenu::reachable_from(options_button)` to the
   `NodeBundle` containing all the options menu `Focusable` entities.
This may look like this:
```rust
let menu = NodeBundle {
    style: Style { flex_direction: Column, ..Default::default()},
    ..Default::default()
};
let button = FocusableButtonBundle::from(ButtonBundle {
    material: Color::rgb(1.0, 0.3, 1.0).into(),
    ..Default::default()
});
let mut spawn_button = |bundle: &FocusableButtonBundle| {
      commands.spawn_bundle(bundle.clone()).id()
};
let options_button = spawn(&button);

// Spawn the game menu                              !!vvvvvv!!
let game_menu = commands.spawn(menu).insert(NavMenu::root()).id();
commads.entity(options_button).insert(Parent(game_menu));
commads.entity(game_button).insert(Parent(game_menu));
commads.entity(quit_button).insert(Parent(game_menu));
commads.entity(load_button).insert(Parent(game_menu));

// Spawn the options menu
let options_menu = commands
      .spawn(menu)
      //              !!vvvvvvvvvvvvvvvvvvvvvvvvvvvvvv!!
      .insert(NavMenu::reachable_from(options_button))
      .id();
commads.entity(graphics_option_button).insert(Parent(options_menu));
commads.entity(audio_options_button).insert(Parent(options_menu));
commads.entity(input_options_button).insert(Parent(options_menu));
commads.entity(gameplay_options_button).insert(Parent(options_menu));
```

With this, your game menu will be isolated from your options menu, you can only
access it by sending the `NavRequest::Action` when `options_button` is focused,
or by sending a `NavRequest::FocusOn(input_options_button)`. 

Specifically, navigation between `Focusable` entities will be constrained to
other `Focusable` that are children of the same `NavMenu`. It creates a
self-contained menu.

### `NavRequest::FocusOn`

You can't directly manipulate which entity is focused, because we need to keep
track of a lot of thing on the backend to make the navigation work as expected.
But you can set the focused element to any arbitrary entity with the
`NavRequest::FocusOn` request.

### `NavMenu` settings

A `NavMenu` doesn't only define menu-to-menu navigation, but it also gives you
finner-grained control on how navigation is handled within a menu.
`NavMenu::{cycle, closed}` controls whether or not going left from the
leftmost element goes to the rightmost and vis-versa. `NavMenu::scope`
sets the menu as a sort of global control menu. It catches `NavRequest::ScopeMove`
requests even when the focused entity is in another sub-menu reachable from this
menu. This behaves like you would expect a tabbed menu to behave. See the
["ultimate" menu navigation
example](https://github.com/nicopap/ui-navigation/blob/master/examples/ultimate_menu_navigation.rs)
for a demonstration.

## Changelog

* `0.8.2`: Fix offsetting of mouse focus with `UiCamera`s with a transform set
  to anything else than zero.
* `0.9.0`: Add `Focusable::cancel` (see documentation for details); Add warning
  message rather than do dumb things when there is more than a single `NavRequest`
  per frame

# License

Copyright © 2021 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure. See
LICENSE file for details.
