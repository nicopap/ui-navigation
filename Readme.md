# Bevy UI navigation

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
<!--[![Latest version](https://img.shields.io/crates/v/bevy_ui_navigation.svg)](https://crates.io/crates/bevy_ui_navigation)-->
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)
<!--[![Documentation](https://docs.rs/bevy_ui_navigation/badge.svg)](https://docs.rs/bevy_ui_navigation)-->

A generic UI navigation algorithm meant to be adaptable to any UI library, but
currently limiting itself to targeting the Bevy engine default UI library.

This crate tracks bevy-main. I do not intend to backport it to bevy 0.5. 

Therefore you cannot get it from [crates.io](https://crates.io). You
must specify it as a git dependency in your `Cargo.toml`:
```toml
[dependencies]
bevy-ui-navigation = { git = "https://github.com/nicopap/ui-navigation" } 
```

It will be available in crates.io when bevy 0.6 comes out.

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
scheme, by sending requests to the `NavRequest` event queue. Check the examples
for a way to send `NavRequest`s based on input. [Here is the basic
implementation for controller support](https://github.com/nicopap/ui-navigation/blob/caa90579429f4948e505c9e81cbd6a972f4a30b3/examples/ultimate_menu_navigation.rs#L62).

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
    material: materials.add(Color::rgb(1.0, 0.3, 1.0).into()),
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

## Roadmap

- [X] Basic flat hierarchy 2D navigation (requires element location deduction)
- [X] 2D navigation jumping through nested node bounds
- [X] Cleanup noisy `Component`s. I think instead of having a `Focused`,
      `Focusable`, `NavNode` and `Active`, we can just have all of those as
      fields of `Focusable`. This probably also reduces massively the number of
      arguments I pass around in the functions I call…
- [X] Hierarchical navigation with Action/Cancel (requires tree layer without
      an active trail)
- [X] Hierarchical navigation with Action/Cancel **with downward focus memory**
- [X] `NavRequest::FocusOn` support
- [X] Do not climb the navigation tree on failed `NavRequest::Move`.
- [X] Remove distinction between `Uncaught` and `Caught` events.
- [X] Tabbed navigation demo (requires Forward/Backward commands support)
- [X] Complex hierarchy with focus memory (requires tree)
- [X] Add more lööps, brother
- [X] Remove "generic" crate
- [X] Minor refactor of `resolve` function + Add FocusableButtonBundle to
      examples to simplify them greatly
- [ ] Replace most calls to `.iter().find(…)` for child non_inert by checking
      the `NavMenu`'s `non_inert_child` rather than `query.nav_menus`. This
      fixes the most likely hotspot which is the recursive function
      `children_focusables`.
- [ ] Descend the hierarchy on Next and Previous (requires non_inert_child
      otherwise it's going to be very difficult to implement)

# License

Copyright © 2021 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure. See
LICENSE file for details.
