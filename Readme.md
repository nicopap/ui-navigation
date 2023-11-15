# Bevy UI navigation

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![Latest version](https://img.shields.io/crates/v/bevy_ui_navigation.svg)](https://crates.io/crates/bevy_ui_navigation)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)
[![Documentation](https://docs.rs/bevy-ui-navigation/badge.svg)](https://docs.rs/bevy-ui-navigation/)

A generic UI navigation algorithm for the
[Bevy](https://github.com/bevyengine/bevy) engine default UI library.

```toml
[dependencies]
bevy-ui-navigation = "0.33.0"
```

The in-depth design specification is [available here][rfc41].

### Examples

Check out the [`examples`][examples] directory for bevy examples.

![Demonstration of "Ultimate navigation" example](https://user-images.githubusercontent.com/26321040/141612751-ba0e62b2-23d6-429a-b5d1-48b09c10d526.gif)


## Cargo Features

This crate exposes the `cuicui_dsl` feature. Disabled by default. Enabling it
will add the `dsl` module, defining `NavigationDsl` useable with the `dsl!`
macro.

This crate exposes the `bevy_ui` feature. It is enabled by default. Toggling
off this feature let you compile this crate without requiring the bevy `render`
feature, however, it requires implementing your own input handling. Check out
the source code for the [`systems`][module-systems] module for leads on
implementing your own input handling.

This crate exposes the `pointer_focus` feature. It is enabled by default.
Disabling it will remove mouse support, and remove the `bevy_mod_picking`
dependency.

## Usage

See [this example][example-simple] for a quick start guide.

[The crate documentation is extensive][doc-root], but for practical reason
doesn't include many examples. This page contains most of the doc examples,
you should check the [examples directory][examples] for examples showcasing
all features of this crate.


### Simple case

To create a simple menu with navigation between buttons, simply replace usages
of [`ButtonBundle`] with [`FocusableButtonBundle`].

You will need to create your own system to change the color of focused elements, and add
manually the input systems, but with that setup you get: **Complete physical
position based navigation with controller, mouse and keyboard. Including rebindable
mapping**.

```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::prelude::*;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultNavigationPlugins)
        .run();
}
```

Use the [`InputMapping`] resource to change keyboard and gamepad button mapping.

If you want to change entirely how input is handled, you should do as follow. All
interaction with the navigation engine is done through
[`EventWriter<NavRequest>`][`NavRequest`]:

```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::prelude::*;

fn custom_input_system_emitting_nav_requests(mut events: EventWriter<NavRequest>) {
    // handle input and events.send(NavRequest::FooBar)
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, NavigationPlugin::new()))
        .add_systems(Update, custom_input_system_emitting_nav_requests)
        .run();
}
```

Check the [`examples directory`][examples] for more example code.

`bevy-ui-navigation` provides a variety of ways to handle navigation actions.
Check out the [`NavEventReaderExt`][module-event_helpers] trait
(and the `NavEventReader` struct methods) for what you can do.

```rust
use bevy::{app::AppExit, prelude::*};
use bevy_ui_navigation::prelude::*;

#[derive(Component)]
enum MenuButton {
    StartGame,
    ToggleFullscreen,
    ExitGame,
    Counter(i32),
    //.. etc.
}

fn handle_nav_events(
    mut buttons: Query<&mut MenuButton>,
    mut events: EventReader<NavEvent>,
    mut exit: EventWriter<AppExit>
) {
    // Note: we have a closure here because the `buttons` query is mutable.
    // for immutable queries, you can use `.activated_in_query` which returns an iterator.
    // Do something when player activates (click, press "A" etc.) a `Focusable` button.
    events.nav_iter().activated_in_query_foreach_mut(&mut buttons, |mut button| match &mut *button {
        MenuButton::StartGame => {
            // start the game
        }
        MenuButton::ToggleFullscreen => {
            // toggle fullscreen here
        }
        MenuButton::ExitGame => {
            exit.send(AppExit);
        }
        MenuButton::Counter(count) => {
            *count += 1;
        }
        //.. etc.
    })
}
```

The focus navigation works across the whole UI tree, regardless of how or where
you've put your focusable entities. You just move in the direction you want to
go, and you get there.

Any [`Entity`] can be converted into a focusable entity by adding the [`Focusable`]
component to it. To do so, just:
```rust
# use bevy::prelude::*;
# use bevy_ui_navigation::prelude::Focusable;
fn system(mut cmds: Commands, my_entity: Entity) {
    cmds.entity(my_entity).insert(Focusable::default());
}
```
That's it! Now `my_entity` is part of the navigation tree. The player can select
it with their controller the same way as any other [`Focusable`] element.

You probably want to render the focused button differently than other buttons,
this can be done with the [`Changed<Focusable>`][Changed] query parameter as follow:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::prelude::{FocusState, Focusable};

fn button_system(
    mut focusables: Query<(&Focusable, &mut BackgroundColor), Changed<Focusable>>,
) {
    for (focus, mut color) in focusables.iter_mut() {
        let new_color = if matches!(focus.state(), FocusState::Focused) {
            Color::RED
        } else {
            Color::BLACK
        };
        *color = new_color.into();
    }
}
```

### Snappy feedback

You will want the interaction feedback to be snappy. This means the
interaction feedback should run the same frame as the focus change. For this to
happen every frame, you should add `button_system` to your app using the
[`NavRequestSystem`] label like so:
```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::prelude::{NavRequestSystem, NavRequest, NavigationPlugin};

fn custom_mouse_input(mut events: EventWriter<NavRequest>) {
    // handle input and events.send(NavRequest::FooBar)
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, NavigationPlugin::new()))
        // ...
        .add_systems(Update, (
            // Add input systems before the focus update system
            custom_mouse_input.before(NavRequestSystem),
            // Add the button color update system after the focus update system
            button_system.after(NavRequestSystem),
        ))
        // ...
        .run();
}
// Implementation from earlier
fn button_system() {}
```


## More complex use cases

### Locking

If you need to supress the navigation algorithm temporarily, you can declare a
[`Focusable`] as [`Focusable::lock`].

This is useful for example if you want to implement custom widget with their
own controls, or if you want to disable menu navigation while in game. To
resume the navigation system, you'll need to send a [`NavRequest::Free`].


### `NavRequest::FocusOn`

You can't directly manipulate which entity is focused, because we need to keep
track of a lot of thing on the backend to make the navigation work as expected.
But you can set the focused element to any arbitrary `Focusable` entity with
[`NavRequest::FocusOn`].

```rust
use bevy::prelude::*;
use bevy_ui_navigation::prelude::NavRequest;

fn set_focus_to_arbitrary_focusable(
    entity: Entity,
    mut requests: EventWriter<NavRequest>,
) {
    requests.send(NavRequest::FocusOn(entity));
}
```

### Set the first focused element

You probably want to be able to chose which element is the first one to gain
focus. By default, the system picks the first [`Focusable`] it finds. To change
this behavior, spawn a prioritized [`Focusable`] with [`Focusable::prioritized`].

### `MenuBuilder`

Suppose you have a more complex game with menus sub-menus and sub-sub-menus etc.
For example, in your everyday 2021 AAA game, to change the antialiasing you
would go through a few menus:
```text
game menu → options menu → graphics menu → custom graphics menu → AA
```
In this case, you need to be capable of specifying which button in the previous
menu leads to the next menu (for example, you would press the "Options" button
in the game menu to access the options menu).

For that, you need to use [`MenuBuilder`].

The high level usage of [`MenuBuilder`] is as follow:
1. First you need a "root" menu using `MenuBuilder::Root`.
2. You need to spawn into the ECS your "options" button with a [`Focusable`]
   component. To link the button to your options menu, you need to do one of
   the following:
   * Add a [`Name("opt_btn_name")`][Name] component in addition to the
     [`Focusable`] component to your options button.
   * Pre-spawn the options button and store somewhere it's [`Entity` id][entity-id]
     (`let opt_btn = commands.spawn(FocusableButtonBundle).id();`)
3. to the `NodeBundle` containing all the options menu [`Focusable`] entities,
   you add the following component:
   * [`MenuBuilder::from_named("opt_btn_name")`][MenuBuilder::reachable_from_named]
     if you opted for adding the `Name` component.
   * [`MenuBuilder::EntityParent(opt_btn)`][MenuBuilder::reachable_from]
     if you have an [`Entity`] id.

In code, This will look like this:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::prelude::{Focusable, MenuSetting, MenuBuilder};
use bevy_ui_navigation::components::FocusableButtonBundle;

struct SaveFile;
impl SaveFile {
    fn bundle(&self) -> impl Bundle {
        // UI bundle to show this in game
        NodeBundle::default()
    }
}
fn spawn_menu(mut cmds: Commands, save_files: Vec<SaveFile>) {
    let menu_node = NodeBundle {
        style: Style { flex_direction: FlexDirection::Column, ..Default::default()},
        ..Default::default()
    };
    let button = FocusableButtonBundle::from(ButtonBundle {
        background_color: Color::rgb(1.0, 0.3, 1.0).into(),
        ..Default::default()
    });
    let mut spawn = |bundle: &FocusableButtonBundle, name: &'static str| {
          cmds.spawn(bundle.clone()).insert(Name::new(name)).id()
    };
    let options = spawn(&button, "options");
    let graphics_option = spawn(&button, "graphics");
    let audio_options = spawn(&button, "audio");
    let input_options = spawn(&button, "input");
    let game = spawn(&button, "game");
    let quit = spawn(&button, "quit");
    let load = spawn(&button, "load");

    // Spawn the game menu
    cmds.spawn(menu_node.clone())
        // Root Menu                 vvvvvvvvvvvvvvvvv
        .insert((MenuSetting::new(), MenuBuilder::Root))
        .push_children(&[options, game, quit, load]);

    // Spawn the load menu
    cmds.spawn(menu_node.clone())
        // Sub menu accessible through the load button
        //                           vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
        .insert((MenuSetting::new(), MenuBuilder::EntityParent(load)))
        .with_children(|cmds| {
            // can only access the save file UI nodes from the load menu
            for file in save_files.iter() {
                cmds.spawn(file.bundle()).insert(Focusable::default());
            }
        });

    // Spawn the options menu
    cmds.spawn(menu_node)
        // Sub menu accessible through the "options" button
        //                           vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
        .insert((MenuSetting::new(), MenuBuilder::from_named("options")))
        .push_children(&[graphics_option, audio_options, input_options]);
}
```

With this, your game menu will be isolated from your options menu, you can only
access it by sending [`NavRequest::Action`] when `options_button` is focused, or
by sending a [`NavRequest::FocusOn(entity)`][`NavRequest::FocusOn`] where `entity`
is any of `graphics_option`, `audio_options` or `input_options`.

Note that you won't need to manually send the [`NavRequest`] if you are using one
of the default input systems provided in the [`systems` module][module-systems].

Specifically, navigation between [`Focusable`] entities will be constrained to other
[`Focusable`] that are children of the same [`MenuSetting`]. It creates a self-contained
menu.

### Types of `MenuSetting`s

To define a menu, you need both the `MenuBuilder` and `MenuSetting` components.

A [`MenuSetting`] gives you fine-grained control on how navigation is handled within a menu:
* `MenuSetting::new().wrapping()` enables looping
  navigation, where going offscreen in one direction "wraps" to the opposite
  screen edge.
* `MenuSetting::new().scope()` creates a "scope" menu that catches [`NavRequest::ScopeMove`]
  requests even when the focused entity is in another sub-menu reachable from this
  menu. This behaves like you would expect a tabbed menu to behave.

See the [`MenuSetting`] documentation or the ["ultimate" menu navigation
example][example-ultimate] for details.


#### Marking

If you need to know from which menu a [`NavEvent::FocusChanged`] originated, you
can use `NavMarker` in the [`mark`][module-marking] module.

A usage demo is available in [the `marking.rs` example][example-marking].


### Menu action with keyboard return (enter) key

The default [`InputMapping`] key to trigger menu actions is the space key.
To use the return key, change the `key_action` attribute.

Otherwise, if you are not using default input handling, add this system:

```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::prelude::{NavRequest, NavRequestSystem};

fn main() {
    App::new()
        // ...
        .add_systems(Update, (
            return_trigger_action.before(NavRequestSystem),
        ));
}

fn return_trigger_action(mut requests: EventWriter<NavRequest>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Return) {
        requests.send(NavRequest::Action);
    }
}
```

[`ButtonBundle`]: https://docs.rs/bevy/latest/bevy/ui/entity/struct.ButtonBundle.html
[Changed]: https://docs.rs/bevy/latest/bevy/ecs/prelude/struct.Changed.html
[doc-root]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/
[`Entity`]: https://docs.rs/bevy/latest/bevy/ecs/entity/struct.Entity.html
[entity-id]: https://docs.rs/bevy/latest/bevy/ecs/system/struct.EntityCommands.html#method.id
[example-marking]: https://github.com/nicopap/ui-navigation/tree/master/examples/marking.rs
[examples]: https://github.com/nicopap/ui-navigation/tree/master/examples
[example-simple]: https://github.com/nicopap/ui-navigation/tree/master/examples/simple.rs
[example-ultimate]: https://github.com/nicopap/ui-navigation/blob/master/examples/ultimate_menu_navigation.rs
[`FocusableButtonBundle`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/components/struct.FocusableButtonBundle.html
[`Focusable::block`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html#method.block
[`Focusable::prioritized`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html#method.prioritized
[`Focusable`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html
[`Focusable::lock`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html#method.lock
[`InputMapping`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/systems/struct.InputMapping.html
[module-event_helpers]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/trait.NavEventReaderExt.html
[module-marking]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/mark/index.html
[module-systems]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/systems/index.html
[Name]: https://docs.rs/bevy/latest/bevy/core/enum.Name.html
[`NavEvent::FocusChanged`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavEvent.html#variant.FocusChanged
[`NavEvent`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavEvent.html
[`MenuSetting`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuSetting.html
[`MenuBuilder`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuBuilder.html
[MenuBuilder::reachable_from]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuBuilder.html#variant.EntityParent
[MenuBuilder::reachable_from_named]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuBuilder.html#method.from_named
[`NavRequest`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html
[`NavRequest::Action`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html#variant.Action
[`NavRequest::FocusOn`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html#variant.FocusOn
[`NavRequest::Free`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html#variant.Unlock
[`NavRequest::Unlock`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html#variant.Unlock
[`NavRequest::ScopeMove`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html#variant.ScopeMove
[`NavRequestSystem`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/struct.NavRequestSystem.html
[rfc41]: https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md

### Chagelog

See the changelog at <https://github.com/nicopap/ui-navigation/blob/master/CHANGELOG.md>

### Version matrix

| bevy | latest supporting version      |
|------|--------|
| 0.12 | 0.33.0 |
| 0.11 | 0.32.0 |
| 0.10 | 0.24.1 |
| 0.9  | 0.23.1 |
| 0.8  | 0.21.0 |
| 0.7  | 0.18.0 |
| 0.6  | 0.14.0 |

# License

Copyright © 2022 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure. See
licenses directory for details.
