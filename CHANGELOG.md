# 0.33.1

Upgrade `fastrand` dev-dependency to `2.0.1`.

# 0.33.0

**BREAKING**:
* Upgrade to `cuicui v0.12.0`
* Upgrade to `bevy v0.12.0`

# 0.32.0

**BREAKING**:
* Upgrade to `cuicui_dsl v0.10.0`
* Add `cuicui_chirp` support
* Add `ParseDsl` impl for `NavigationDsl`

# 0.31.0

Fix examples link in Readme.

# 0.31.0

**BREAKING**:
* Feature-gate default mouse handling behind `pointer_focus`
* Use `bevy_mod_picking` instead of custom hard-coded system to emit `FocusOn`s
  from mouse events.
* This should add out-of-the-box support for touch inputs & handles complex
  UI trees much better. Including not picking stuff when using `bevy-inspector-egui`
  (if you enable the `bevy_mod_picking/backend_egui` feature)
* Removed all mouse-related systems and types in `systems`.
  * Consider using `bevy_mod_picking` instead.

# 0.30.0

Handle gracefully when `MenuBuilder::NamedParent` can't find an `Focusable`
with the given name. Now instead of giving up, it retries next frame to find
`Focusable`s with the provided name.

# 0.29.0

Refelct and register `MenuBuilder`.

# 0.28.0

Bump `cuicui_dsl` version to `0.8.1`.

# 0.27.0

Non-breaking: Add a `cuicui_dsl` `DslBundle` behind the `cuicui_dsl` feature flag.

# 0.26.0

**BREAKING**: Fix the `bevy_ui` feature. Ooops sorry.

# 0.25.0

**BREAKING**: Update ot bevy 0.11.0

# 0.24.1

* Fix the `ultimate_menu_navigation.rs` example
* add keyboard navigation to it, `too_many_focusables.rs`, `menu_navigation.rs` and `simple.rs`.
* Add focus follow mouse to `simple.rs` and `ultimate_menu_navigation.rs`
* Add `bevy_framepace` to all examples.
* Remove `#[bundle]` attribute from navigation bundles (it's now useless)

# 0.24.0

* Improve performance on `NavEventReader::activated_in_query_foreach_mut`
* **BREAKING**: Update to bevy 0.10.0

# 0.23.1

Fix docs.rs `rustdoc-scrape-examples` flags.

# 0.23.0

Start porting back to this crate all the changes made in [the RFC PR]
* Add (optional) `Reflect` derive to all navigation components, it's on by default,
  disable it using `--no-default-features --features "bevy-ui-navigation/bevy_ui"`
* Add a bunch of tests
* Re-order `TreeMenu` insertion, the transformation from `MenuBuilder` to the
  internally used component (`TreeMenu`) is now done in `PreUpdate` instead of
  `PostUpdate`. This fixes a potential frame lag.

# 0.22.0

Update to bevy 0.9.0

# 0.21.0

Add the [`NavEventReader::types`] method

# 0.20.0

Improve lock system:
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

# 0.19.0

**Breaking**: Update to bevy 0.8.0
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

# 0.18.0

* **Breaking**: Remove marker generic type parameter from `systems::NodePosQuery`.
  The `generic_default_mouse_input` system now relies on the newly introduced
  [`ScreenBoundaries`] that abstracts camera offset and zoom settings.
* **Important**: Introduced new plugins to make it even simpler to get started.
* Add `event_helpers` module introduction to README.
* Fix `bevy-ui` feature not building. This issue was introduced in `0.16.0`.

# 0.17.0

Non-breaking, but due to cargo semver handling is a minor bump.
* Add the `event_helpers` module to simplify ui event handling
* Fix README and crate-level documentation links

# 0.16.0

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

# 0.15.1

Fix the `marker` systems panicking at startup.

# 0.15.0

**Breaking**: bump bevy version to `0.7` (you should be able to
upgrade from `0.14.0` without changing your code)

# 0.14.0

Some important changes, and a bunch of new very useful features.
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

# 0.13.1

Fix broken URLs in Readme.md

# 0.13.0

Complete rewrite of the [`NavMenu`] declaration system:
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

# 0.12.1

Add *by-name* menus, making declaring complex menus in one go much easier.

# 0.12.0

Remove `NavMenu` methods from `MarkingMenu` and make the `menu`
field public instead. Internally, this represented too much duplicate code.

# 0.11.1

Add the `marker` module, enabling propagation of user-specified
components to `Focusable` children of a `NavMenu`.

# 0.11.0

Add the `Focusable::lock` feature. A focusable now can be declared
as "lock" and block the ui navigation systems until the user sends a
`NavRequest::Free`. See the `locking.rs` example for illustration.
* Breaking: New enum variants on `NavRequest` and `NavEvent`

# 0.10.0

Add the `bevy-ui` feature, technically this includes breaking
changes, but it is very unlikely you need to change your code to get it
working:
* **Breaking**: if you were manually calling `default_mouse_input`, it now has
  additional parameters
* **Breaking**: `ui_focusable_at` and `NodePosQuery` now have type parameters

# 0.9.1

Fix #8, Panic on diagonal gamepad input

# 0.9.0

Add [`Focusable::cancel`] (see documentation for details); Add warning
message rather than do dumb things when there is more than a single [`NavRequest`]
per frame

# 0.8.2

Fix offsetting of mouse focus with `UiCamera`s with a transform set
to anything else than zero.


[diff-18-19]: https://github.com/nicopap/ui-navigation/compare/v0.18.0...v0.19.0
[`Focusable::cancel`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html#method.cancel
[`InputMapping::keyboard_navigation`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/systems/struct.InputMapping.html#structfield.keyboard_navigation
[`LockReason`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/enum.LockReason.html
[`NavEventReader::types`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/struct.NavEventReader.html#method.types
[`NavMenu`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/menu/enum.MenuSetting.html
[`NavRequest`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavRequest.html
[`ScreenBoundaries`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/custom/struct.ScreenBoundaries.html
[the RFC PR]: https://github.com/bevyengine/bevy/pull/5378
[`NavEvent::InitiallyFocused`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/events/enum.NavEvent.html#variant.InitiallyFocused
[`Focusable`]: https://docs.rs/bevy-ui-navigation/latest/bevy_ui_navigation/prelude/struct.Focusable.html
[pr-hover]: https://github.com/nicopap/bevy/blob/0530d03b514e5e1e3d42a89283b5e6d050e9c265/crates/bevy_ui/src/focus.rs#L190-L223
[rfc41]: https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md
