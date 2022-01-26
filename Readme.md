# Bevy UI navigation

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![Latest version](https://img.shields.io/crates/v/bevy_ui_navigation.svg)](https://crates.io/crates/bevy_ui_navigation)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)
[![Documentation](https://docs.rs/bevy-ui-navigation/badge.svg)](https://docs.rs/bevy-ui-navigation/)

A generic UI navigation algorithm meant to be adaptable to any UI library, but
currently limiting itself to targeting the Bevy engine default UI library.

```toml
[dependencies]
bevy-ui-navigation = "0.11.1"
```

The in-depth design specification is [available here](https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md).

### Examples

Check out the [`examples`](https://github.com/nicopap/ui-navigation/tree/master/examples) directory for bevy examples.

Ultimate navigation example demo
![Demonstration of "Ultimate navigation"
example](https://user-images.githubusercontent.com/26321040/141612751-ba0e62b2-23d6-429a-b5d1-48b09c10d526.gif)


## Cargo Features

This crate exposes the `bevy-ui` feature. It is enabled by default. Toggling
off this feature let you compile this crate without requiring the bevy `render`
feature. But you won't be able to use [`FocusableButtonBundle`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/components/struct.FocusableButtonBundle.html), and you'll have
to use [`generic_default_mouse_input`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/systems/fn.generic_default_mouse_input.html) for mouse input and define special spacial
components to get it working.


## Usage

See [this example](https://github.com/nicopap/ui-navigation/blob/master/examples/flat_2d_nav.rs)
for a quick start guide.


### Simple case

You just have a collection of buttons displayed on screen and you want the
player to be able to select between them with their controller? Simply use the
[`bevy_ui_navigation::components::FocusableButtonBundle`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/components/struct.FocusableButtonBundle.html) instead of
`ButtonBundle`. The navigation system is capable of switching focus based on
the 2D position of each focusable element.

This won't work out of the box though, you must wire the UI to the control
scheme, by sending requests to the [`NavRequest`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavRequest.html) event queue. We do not provide
out of the box a way to do this, but we provide default input handling systems.
Try this:
```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::systems::{default_gamepad_input, default_keyboard_input, InputMapping};
use bevy_ui_navigation::NavigationPlugin;
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
are modifiable with the
[`InputMapping`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/systems/struct.InputMapping.html) resource. Check out the doc for it for
more details.

Check the [`examples`](https://github.com/nicopap/ui-navigation/tree/master/examples) directory for more example code.

To respond to relevant user input, for example when the player pressed the
"Action" button when focusing `start_game_button`, you should read the
[`NavEvent`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavEvent.html) event queue:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::{NavEvent, NavRequest};

struct Gameui {
    start_game_button: Entity,
}

fn handle_nav_events(mut events: EventReader<NavEvent>, game: Res<Gameui>) {
    use bevy_ui_navigation::{NavEvent::NoChanges, NavRequest::Action};
    for event in events.iter() {
        match event {
            NoChanges { from, request: Action } if *from.first() == game.start_game_button => {
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

In truth,
[`FocusableButtonBundle`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/components/struct.FocusableButtonBundle.html) is just a `ButtonBundle` with an extra
[`Focusable`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html) component field.

Any [`Entity`](https://docs.rs/bevy/latest/bevy/ecs/entity/struct.Entity.html) can be converted into a focusable entity by adding the [`Focusable`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html)
component to it. To do so, just:
```rust
# use bevy::prelude::*;
# use bevy_ui_navigation::Focusable;
fn system(mut cmds: Commands, my_entity: Entity) {
    cmds.entity(my_entity).insert(Focusable::default());
}
```
That's it! Now `my_entity` is part of the navigation tree. The player can
select it with their controller the same way as any other [`Focusable`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html) element.

You probably want to render the focused button differently than other buttons,
this can be done with the [`Changed<Focusable>`](https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.Changed.html) query parameter as follow:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::Focusable;

fn button_system(
    mut focusables: Query<(&Focusable, &mut UiColor), Changed<Focusable>>,
) {
    for (focus_state, mut color) in focusables.iter_mut() {
        let new_color = if focus_state.is_focused() {
            Color::RED
        } else {
            Color::BLUE
        };
        *color = new_color.into();
    }
}
```

## More complex use cases

### With `NavMenu`s

Suppose you have a more complex game with menus sub-menus and sub-sub-menus etc.
For example, in your everyday 2021 AAA game, to change the antialiasing you
would go through a few menus:
```text
game menu → options menu → graphics menu → custom graphics menu → AA
```
In this case, you need to be capable of specifying which button in the previous
menu leads to the next menu (for example, you would press the "Options" button
in the game menu to access the options menu).

For that, you need to use the
[`NavMenu`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.NavMenu.html) component.
1. First you need a "root"
   [`NavMenu`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.NavMenu.html). This is a limit of the navigation
   algorithm (which may be fixed in the future)
2. You also need the `Entity` value of your "Options" button
3. You then add a
   [`NavMenu::reachable_from(options_button)`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.NavMenu.html#method.reachable_from) to the
   `NodeBundle` containing all the options menu [`Focusable`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html) entities.
This may look like this:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::NavMenu;
use bevy_ui_navigation::components::FocusableButtonBundle;

fn spawn_menu(mut cmds: Commands) {
    let menu = NodeBundle {
        style: Style { flex_direction: FlexDirection::Column, ..Default::default()},
        ..Default::default()
    };
    let button = FocusableButtonBundle::from(ButtonBundle {
        color: Color::rgb(1.0, 0.3, 1.0).into(),
        ..Default::default()
    });
    let mut spawn = |bundle: &FocusableButtonBundle| {
          cmds.spawn_bundle(bundle.clone()).id()
    };
    let options = spawn(&button);
    let graphics_option = spawn(&button);
    let audio_options = spawn(&button);
    let input_options = spawn(&button);
    let game = spawn(&button);
    let quit = spawn(&button);
    let load = spawn(&button);

    // Spawn the game menu, with a `NavMenu`
    let game_menu = cmds
        .spawn_bundle(menu.clone())
        //      vvvvvvvvvvvvv
        .insert(NavMenu::root())
        .push_children(&[options, game, quit, load])
        .id();

    // Spawn the options menu
    let options_menu = cmds
        .spawn_bundle(menu)
        //             !!vvvvvvvvvvvvvvvvvvvvvvv!!
        .insert(NavMenu::reachable_from(options))
        .push_children(&[graphics_option, audio_options, input_options])
        .id();
}
```


With this, your game menu will be isolated from your options menu, you can only
access it by sending the [`NavRequest::Action`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavRequest.html#variant.Action) when `options_button` is focused,
or by sending a
[`NavRequest::FocusOn(input_options_button)`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavRequest.html#variant.FocusOn).

Specifically, navigation between [`Focusable`] entities will be constrained to
other [`Focusable`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html) that are children of the same [`NavMenu`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.NavMenu.html). It creates a
self-contained menu.

### `NavRequest::FocusOn`

You can't directly manipulate which entity is focused, because we need to keep
track of a lot of thing on the backend to make the navigation work as expected.
But you can set the focused element to any arbitrary entity with the
[`NavRequest::FocusOn`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavRequest.html#variant.FocusOn) request.

### `NavMenu` settings

A `NavMenu` doesn't only define menu-to-menu navigation, but it also gives you
finner-grained control on how navigation is handled within a menu.
`NavMenu::{cycle, closed}` controls whether or not going left from the
leftmost element goes to the rightmost and vis-versa.
[`NavMenu::scope`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.NavMenu.html#method.scope)
sets the menu as a sort of global control menu. It catches [`NavRequest::ScopeMove`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavRequest.html#variant.ScopeMove)
requests even when the focused entity is in another sub-menu reachable from this
menu. This behaves like you would expect a tabbed menu to behave. See the
["ultimate" menu navigation
example](https://github.com/nicopap/ui-navigation/blob/master/examples/ultimate_menu_navigation.rs)
for a demonstration.

### Marking

If you need to know from which menu a
[`NavEvent::FocusChanged`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavEvent.html#variant.FocusChanged) originated, you may want to use the [`marker` module](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/marker/index.html).

A usage demo is available in [the `marking.rs` example](https://github.com/nicopap/ui-navigation/tree/master/examples/marking.rs)

### Locking

If you need to supress the navigation algorithm temporarily, you can declare a
`Focusable` as
[`Focusable::lock`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html#method.lock).

This is useful for example if you want to implement custom widget with their
own controls, or if you want to disable menu navigation while in game. To
resume the navigation system, you'll need to send a
[`NavRequest::Free`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/enum.NavRequest.html#variant.Free).

## Changelog

* `0.8.2`: Fix offsetting of mouse focus with `UiCamera`s with a transform set
  to anything else than zero.
* `0.9.0`: Add [`Focusable::cancel`](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.Focusable.html#method.cancel) (see documentation for details); Add warning
  message rather than do dumb things when there is more than a single `NavRequest`
  per frame
* `0.9.1`: Fix #8, Panic on diagonal gamepad input
* `0.10.0`: Add the `bevy-ui` feature, technically this includes breaking
  changes, but it is very unlikely you need to change your code to get it
  working 
  * Breaking: if you were manually calling `default_mouse_input`, it now has 
    additional parameters
  * Breaking: `ui_focusable_at` and `NodePosQuery` now have type parameters
* `0.11.0`: Add the `Focusable::lock` feature. A focusable now can be declared
  as "lock" and block the ui navigation systems until the user sends a
  `NavRequest::Free`. See the `locking.rs` example for illustration.
  * Breaking: New enum variants on `NavRequest` and `NavEvent`
* `0.11.1`: Add the [`marker` module](https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/marker/index.html), enabling propagation of user-specified
  components to `Focusable` children of a `NavMenu`.

# License

Copyright © 2021 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure. See
LICENSE file for details.
