# Bevy UI navigation

[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![Latest version](https://img.shields.io/crates/v/bevy_ui_navigation.svg)](https://crates.io/crates/bevy_ui_navigation)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)
[![Documentation](https://docs.rs/bevy-ui-navigation/badge.svg)](https://docs.rs/bevy-ui-navigation/)

A generic UI navigation algorithm for the
[Bevy](https://github.com/bevyengine/bevy) engine default UI library.

```toml
[dependencies]
bevy-ui-navigation = "0.32.0"
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
To use the return key, change the `key_action` attribute, or add this system:

```rust
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


## Changelog

* `0.8.2`: Fix offsetting of mouse focus with `UiCamera`s with a transform set
  to anything else than zero.
* `0.9.0`: Add [`Focusable::cancel`] (see documentation for details); Add warning
  message rather than do dumb things when there is more than a single [`NavRequest`]
  per frame
* `0.9.1`: Fix #8, Panic on diagonal gamepad input
* `0.10.0`: Add the `bevy-ui` feature, technically this includes breaking
  changes, but it is very unlikely you need to change your code to get it
  working
  * **Breaking**: if you were manually calling `default_mouse_input`, it now has
    additional parameters
  * **Breaking**: `ui_focusable_at` and `NodePosQuery` now have type parameters
* `0.11.0`: Add the `Focusable::lock` feature. A focusable now can be declared
  as "lock" and block the ui navigation systems until the user sends a
  `NavRequest::Free`. See the `locking.rs` example for illustration.
  * Breaking: New enum variants on `NavRequest` and `NavEvent`
* `0.11.1`: Add the `marker` module, enabling propagation of user-specified
  components to `Focusable` children of a `NavMenu`.
* `0.12.0`: Remove `NavMenu` methods from `MarkingMenu` and make the `menu`
  field public instead. Internally, this represented too much duplicate code.
* `0.12.1`: Add *by-name* menus, making declaring complex menus in one go much easier.
* `0.13.0`: Complete rewrite of the [`NavMenu`] declaration system:
  * Add automatic submenu access for `scope` menus.
  * Rename examples, they were named weirdly.
  * **Breaking**: Replace `NavMenu` constructor API with an enum (KISS) and a
    set of methods that return various types of `Bundle`s. Each variant does
    what the `cycle` and `scope` methods used to do.
  * **Breaking**: `NavMenu` is not a component anymore, the one used in the
    navigation algorithm is now private, you can't match on `NavMenu` in query
    parameters anymore. If you need that functionality, create your own marker
    component and add it manually to your menu entities.
  * **Breaking**: Remove `is_*` methods from `Focusable`. Please use the
    `state` method instead. The resulting program will be more correct. If you
    are only worried about discriminating the `Focused` element from others,
    just use a `if let Focused = focus.state() {} else {}`. Please see the
    examples directory for usage patterns.
  * **Breaking**: A lot of struct and module reordering to make documentation
    more discoverable. Notably `Direction` and `ScopeDirection` are now in the
    `events` module.
* `0.13.1`: Fix broken URLs in Readme.md
* `0.14.0`: Some important changes, and a bunch of new very useful features.
  * Add a `Focusable::dormant` constructor to specify which focusable you want
    to be the first to focus, this also works for [`Focusable`]s within
    [`NavMenu`]s.
  * **Important**: This changes the library behavior, now there will
    automatically be a `Focused` entity set. Add a system to set the first
    `Focused` whenever `Focusable`s are added to the world.
  * Add [`NavEvent::InitiallyFocused`] to handle this first `Focused` event.
  * Early-return in `default_gamepad_input` and `default_keyboard_input` when
    there are no `Focusable` elements in the world. This saves your precious
    CPU cycles. And prevents spurious `warn` log messages.
  * Do not crash when resolving `NavRequest`s while no `Focusable`s exists in
    the world. Instead, it now prints a warning message.
  * **Important**: Now the focus handling algorithm supports multiple `NavRequest`s
    per frame. If previously you erroneously sent multiple `NavRequest` per
    update and relied on the ignore mechanism, you'll have a bad time.
  * This also means the focus changes are visible as soon as the system ran,
    the new `NavRequestSystem` label can be used to order your system in
    relation to the focus update system. **This makes the focus change much
    snappier**.
  * Rewrite the `ultimate_menu_navigation.rs` without the `build_ui!` macro
    because we shouldn't expect users to be familiar with my personal weird
    macro.
  * **Breaking**: Remove `Default` impl on `NavLock`. The user shouldn't be
    able to mutate it, you could technically overwrite the `NavLock` resource
    by using `insert_resource(NavLock::default())`.
* `0.15.0`: **Breaking**: bump bevy version to `0.7` (you should be able to
    upgrade from `0.14.0` without changing your code)
* `0.15.1`: Fix the `marker` systems panicking at startup.
* `0.16.0`:
  * Cycling now wraps around the screen properly, regardless of UI camera
    position and scale. See the new `off_screen_focusables.rs` example for a demo.
  * Fix the `default_mouse_input` system not accounting for camera scale.
  * Update examples to make use of `NavRequestSystem` label, add more recommendations
    with regard to system ordering.
  * **Warning**: if you have some funky UI that goes beyond the screen (which
    you are likely to have if you use the `Overflow` feature), this might result
    in unexpected behavior. Please fill a bug if you hit that limitation.
  * Add a nice stress test example with 96000 focusable nodes. This crate is not
    particularly optimized, but it's nice to see it holds up!
  * **Breaking**: Removed the undocumented public `UiCamera` marker component, please
    use the bevy native `bevy::ui::entity::CameraUi` instead. Furthermore, the
    `default_mouse_input` system has one less parameter.
  * **Warning**: if you have multiple UI camera, things will definitively break. Please
    fill an issue if you stumble uppon that case.
* `0.17.0`: Non-breaking, but due to cargo semver handling is a minor bump.
  * Add the `event_helpers` module to simplify ui event handling
  * Fix README and crate-level documentation links
* `0.18.0`:
  * **Breaking**: Remove marker generic type parameter from `systems::NodePosQuery`.
    The `generic_default_mouse_input` system now relies on the newly introduced
    [`ScreenBoundaries`] that abstracts camera offset and zoom settings.
  * **Important**: Introduced new plugins to make it even simpler to get started.
  * Add `event_helpers` module introduction to README.
  * Fix `bevy-ui` feature not building. This issue was introduced in `0.16.0`.
* `0.19.0`: **Breaking**: Update to bevy 0.8.0
  * Please look at the [diff][diff-18-19] for the `examples` directory for help on migration.
  * **Important**: Huge API modifications to comply with feedback from [RFC 41][rfc41].
  * **Breaking**: Removed the `event_helpers` module, use instead the `.nav_iter` method
    on `EventReader<NavEvent>`. You should import the `NavEventReaderExt` trait for `.nav_iter`
    to be available on `EventReader<NavEvent>`.
  * **Breaking**: Renamed `dormant` → `prioritized`
  * **Breaking**: Renamed `NavMenu` → `MenuSetting`
    * Instead of an enum, `NavMenu` is now a struct with two boolean fields
  * **Breaking**: Renamed `bundles` → `menu`
    * Note: Dealing with "seeds" (aka bundles) is now much simpler
      and similar to other bevy plugins
    * This is serious breaking change, please check the [`MenuBuilder`] docs
    * Disclaimer: `MenuBuilder` is likely to be renamed in the future.
  * **Breaking**: Add a `prelude` module, for all your crazy folks who like to not name stuff
    they use (such as myself); this replaces the names being available at the top crate level,
    if your code breaks because "bevy_ui_navigation doesn't export this symbol", try importing
    `prelude` instead.
  * **Warning**: 0.8.0 removed the ability for the user to change the ui camera position
    and perspective, see <https://github.com/bevyengine/bevy/pull/5252>
    Generic support for user-defined UIs still allows custom cropping, but it not a relevant
    use case to the default bevy_ui library.
  * Keyboard navigation in the style of games pre-dating computer mouses is now disabled by default.
    While you can still use the escape and tab keys for interaction, you cannot use keyboard keys
    to navigate between focusables anymore, this prevents keyboard input conflicts.
    You can enable keyboard movement using the [`InputMapping::keyboard_navigation`] field.
  * Improved the heuristic to set the first focused element, now it tries to find an element
    in root menus if there is such a thing.
  * Touch input handling has been removed, it was untested and probably broken, better let
    the user who knows what they are doing do it.
  * **NEW**: Add complete user-customizable focus movement. Now it should be possible to implement
    focus navigation in 3d space.
  * **Breaking**: This requires making the plugin generic over the navigation system, if you were
    manually adding `NavigationPlugin`, please consider using `DefaultNavigationPlugins` instead,
    if it is not possible, then use `NavigationPlugin::new()`.
  * **Breaking**: moved the `insert_tree_menus` and `resolve_named_menus` systems to
    `CoreStage::PostUpdate`, which fixes a variety of bugs and unspecified behaviors with
    regard to adding menus and selecting the first focused element.
* `0.20.0`: Improve lock system
  * **Breaking**: Rename `NavRequest::Free` → `NavRequest::Unlock` for consistency.
  * **Breaking**: `NavEvent::Unlocked` now contains a [`LockReason`] rather than an `Entity`.
  * Add `NavRequest::Lock` request to block navigation through a request.
  * Add a way to spawn and set focusables as not focusable at all with [`Focusable::block`].
  * **Breaking**: The default mouse input system
    now by default does not immediately focus on hovered elements.
    This is more in line with conventional UI libraries.
    To keep the old behavior, set the `InputMapping.focus_follows_mouse` field to `true`.
    If you want to have graphical effects on hover, please define your own hover system.
    [Here is how it was done in the bevy merge PR][pr-hover].
* `0.21.0`: Add the [`NavEventReader::types`] method
* `0.22.0`: Update to bevy 0.9.0
* `0.23.0`: Start porting back to this crate all the changes made in [the RFC PR]
  * Add (optional) `Reflect` derive to all navigation components, it's on by default,
    disable it using `--no-default-features --features "bevy-ui-navigation/bevy_ui"`
  * Add a bunch of tests
  * Re-order `TreeMenu` insertion, the transformation from `MenuBuilder` to the
    internally used component (`TreeMenu`) is now done in `PreUpdate` instead of
    `PostUpdate`. This fixes a potential frame lag.
* `0.23.1`: Fix docs.rs `rustdoc-scrape-examples` flags.
* `0.24.0`:
  * Improve performance on `NavEventReader::activated_in_query_foreach_mut`
  * **BREAKING**: Update to bevy 0.10.0
* `0.24.1`:
  - Fix the `ultimate_menu_navigation.rs` example
  - add keyboard navigation to it, `too_many_focusables.rs`, `menu_navigation.rs` and `simple.rs`.
  - Add focus follow mouse to `simple.rs` and `ultimate_menu_navigation.rs`
  - Add `bevy_framepace` to all examples.
  - Remove `#[bundle]` attribute from navigation bundles (it's now useless)
* `0.25.0`: **BREAKING**: Update ot bevy 0.11.0
* `0.26.0`: **BREAKING**: Fix the `bevy_ui` feature. Ooops sorry.
* `0.27.0`: Non-breaking: Add a `cuicui_dsl` `DslBundle` behind the `cuicui_dsl` feature flag.
* `0.28.0`: Bump `cuicui_dsl` version to `0.8.1`.
* `0.29.0`: Refelct and register `MenuBuilder`.
* `0.30.0`: Handle gracefully when `MenuBuilder::NamedParent` can't find an `Focusable`
  with the given name. Now instead of giving up, it retries next frame to find
  `Focusable`s with the provided name.
* `0.31.0`: **BREAKING**:
  * Feature-gate default mouse handling behind `pointer_focus`
  * Use `bevy_mod_picking` instead of custom hard-coded system to emit `FocusOn`s
    from mouse events.
  * This should add out-of-the-box support for touch inputs & handles complex
    UI trees much better. Including not picking stuff when using `bevy-inspector-egui`
    (if you enable the `bevy_mod_picking/backend_egui` feature)
  * Removed all mouse-related systems and types in `systems`.
    * Consider using `bevy_mod_picking` instead.
* `0.31.1`: Fix examples link in Readme.
* `0.32.0`: **BREAKING**:
  * Upgrade to `cuicui_dsl v0.10.0`
  * Add `cuicui_chirp` support
  * Add `ParseDsl` impl for `NavigationDsl`

[the RFC PR]: https://github.com/bevyengine/bevy/pull/5378
[diff-18-19]: https://github.com/nicopap/ui-navigation/compare/v0.18.0...v0.19.0
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
[`Focusable::cancel`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html#method.cancel
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
[`NavEvent::InitiallyFocused`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavEvent.html#variant.InitiallyFocused
[`MenuSetting`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuSetting.html
[`NavMenu`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuSetting.html
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
[`ScreenBoundaries`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/custom/struct.ScreenBoundaries.html
[`LockReason`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/enum.LockReason.html
[pr-hover]: https://github.com/nicopap/bevy/blob/0530d03b514e5e1e3d42a89283b5e6d050e9c265/crates/bevy_ui/src/focus.rs#L190-L223
[`NavEventReader::types`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/struct.NavEventReader.html#method.types

### Version matrix

| bevy | latest supporting version      |
|------|--------|
| 0.11 | 0.32.0 |
| 0.10 | 0.24.1 |
| 0.9  | 0.23.1 |
| 0.8  | 0.21.0 |
| 0.7  | 0.18.0 |
| 0.6  | 0.14.0 |

### Notes on API Stability

In the 4th week of January, there has been 5 breaking version changes. `0.13.0`
marks the end of this wave of API changes. And things should go much slower in
the future.

The new `NavMenu` construction system helps adding orthogonal features to the
library without breaking API changes. However, since bevy is still in `0.*`
territory, it doesn't make sense for this library to leave the `0.*` range.

Also, the way cargo handles versioning for `0.*` crates is in infraction of
the semver specification. Meaning that additional features without breakages
requires bumping the minor version rather than the patch version (as should
pre-`1.` versions do).


# License

Copyright © 2022 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure. See
licenses directory for details.

### Font

The font in `font.ttf` is derived from Adobe SourceSans, licensed
under the SIL OFL. see file at `licenses/SIL Open Font License.txt`.
