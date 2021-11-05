# UI Navigation deisgn

## API

Excellent navigation controls are masterpieces of design and complex to come up
with, yet completely transparent to the user. I'm designing a relatively generic
system that can be brought up in any game and enable the game maker to use it
for their own purpose without too much hassle.

The game should emit `Command` events, such as `MoveLeft`, `MoveUp`, `Back`,
`Next` etc. Our `navigation_system` manages a single focused element and
changes it based on the `Command`s it receives. It emits `Event`s such as
`FocusChange` or `Action` after _resolving_ the command.

`Command`s are just an `enum` of events with semantic meaning, for example, we
can imagine it looking like this:
```rust
enum Command {
    MoveUp, MoveDown,
    MoveLeft, MoveRight,
    Previous, Next,
    Action, Cancel,
}
```

What I want is being able to specify how to navigate a menu (however complex it is)
without headaches. A simple menu should be dead easy to specify it's navigation, while a
complex menu should be possible to specify. Let's start by the simple one.

## Design

Let's start with a practical example, and try figure out how we could specify
navigation in it:

![Typical RPG tabbed menu](https://user-images.githubusercontent.com/26321040/140542742-0618a428-6841-4e64-94f0-d64f2d03d119.png)

You can move UP/DOWN in the submenu, but you see you can also use LT/RT to
go from one submenu to the other (the _soul_, _body_ etc tabs).

Here is a draft of how I could specify in-code how to navigate this menu. (This
uses a syntax close to my macro to specify bevy ui, I hope it's self-explanatory)

```rust
build_ui! {
  vertical {:container "menu"} (
    horizontal(
      tab {:navigable} ("soul")
      tab {:navigable} ("body")
      tab {:navigable} ("mind")
      tab {:navigable} ("all")
    )
    horizontal {:container "soul menu"} (
      vertical(
        panel {:navigable} ("gdk")
        panel {:navigable} ("kfc")
        panel {:navigable} ("abc")
      )
      vertical {:container "abc menu"} (
        title_label("ABC")
        grid(
          token {:navigable} ("a")    token {:navigable} ("A")
          token {:navigable} ("b")    token {:navigable} ("B")
          token {:navigable} ("c")    token {:navigable} ("C")
        )
      )
    )
  )
}
fn react_events(events: ResMut<Event<navig::Event>>, state: Res<Game>) {
  for event in events.iter() {
    if matches!(event, navig::Event::Action(state.ui_elems.start_game)) {
      // start game
    }
    //etc.
  }
}
```

### Basic physical navigation

Let's drop the idea to change submenu with LT/RT for a while. We would change
submenus by navigating upward with the Dpad or arrow keys to the tab bar and
selecting the tab with the LEFT and RIGHT keys.

Typical game engine implementation of menu navigation requires the developer to
specify the "next", "left", "right", "previous" etc.[^1] relationships between
focusable elements. This is just lame! Our game engine not only already knows
the position of our UI elements, but it also has knowledge of the logical
organization of elements through the `Parent`/`Children` relationship. (in this
case, children are specified in the parenthesis following the element name,
look at `grid(...)`)

Therefore, with the little information we provided through this hand wavy
specification, we should already have everything setup for the navigation to
just work™.


### Dimensionality

Let's go back to our ambition of using LT/RT for navigation now. 

UI navigation is not just 2D, it's N-dimensional. The
LT/RT to change tabs is like navigating in a 3rd orthogonal dimension to the
dimensions you navigate with UP DOWN LEFT RIGHT inside the menus.

I'll call those dimensions `Plane`s because it's easier to type than
"Dimension", I'll limit the implementation to 3 planes. We can imagine truly
exotic menu navigation systems with more than 3 dimensions, but I'll not
worry about that. Our `Plane`s are:
* `Plane::Menu`: Use LT/RT to move from one element to the next/previous
* `Plane::Select`: Instead of emitting an `Action` event when left-clicking or
  pressing A/B, go up-down that direction
* `Plane::Physical`: Use the `Transform` positioning and navigate with Dpad or
  arrow keys

Each `Command` moves the focus on a specific `Plane` as follow:
* `Plane::Menu`: `Previous`, `Next`
* `Plane::Select`: `Action`, `Cancel`
* `Plane::Physical`: `MoveUp`, `MoveDown`, `MoveLeft`, `MoveRight`

We should also be able to "loop" the menu, for example going LEFT of "soul"
loops us back to "all" at the other side of the tab bar.

### Specifying navigation dimensions

We posited we could easily infer the physical layout and bake a navigation map
automatically based on the `Transform` positions of the elements. This is not
the case for the dimensions of our navigation. We can't magically infer the
intent of the developer: They need to specify the dimensionality of their menus.

Let's go back to the menu. In the example code, I refer to `:container` and
`:navigable`. I've not explained yet what those are. Let's clear things up.

A _navigable_ is an element that can be focused. A _container_ is a node entity
that contains _navigables_ and 0 or 1 other _container_.

![The previous tabbed menu example, but with _containers_ and _navigables_ highlighted](https://user-images.githubusercontent.com/26321040/140542768-4fdd5f23-2c2e-43c1-9fa4-cc11fe67c619.png)

The _containers_ are represented as semi-transparent squares; the _navigables_
are circles.

In rust, it might look like this:
```rust
struct Container {
  inner: Option<Box<Container>>,
  siblings: NonEmptyVec<Navigable>,
  active: SiblingIndex,
  plane: Plane,
}
```
Note: the ECS implementation relies on `Component`s, so it will be different

For now, let's focus on navigation within a single _container_. The
_navigables_ are collected by walking through the bevy `Parent`/`Children` 
hierarchy until we find a `Parent` with the `Container` component. Transitive
children[^2] `Entity`s marked with `Navigable` are all _navigable siblings_.
When collecting sibling, we do not traverse _container_ boundaries (ie: the
_navigables_ of a contain**ed** _container_ are not the _navigables_ of the
contain**ing** _container_)

A `Container`'s plane field specifies how to navigate between it's contained
siblings. In the case of our menu, it would be `Plane::Menu`. By default it is
`Plane::Physical`.

So how does that play with `Command`s?  For example: You are focused on "B"
navigable in the "abc menu" container, and you issue a `Next` `Command` (press
RT). What happens is this:
1. What is the `plane` of "abc menu"? It is `Physical`, I must look up the
   containing `Container`'s plane.
2. What is the `plane` of "soul menu"? It is `Physical`, I must look up the
   containing `Container`'s plane.
3. What is the `plane` of "menu"? It is `Menu`!
4. Let's take it's current active _navigable_ and look which other _navigable_
   we can reach with our `Command`

In short, if your focused element is inside a given _container_ and you emit a `Command`, 
you climb up the container hierarchy until you find one in the plane of your
`Command`, then lookup the sibling you can reach with the given `Command`.

This algorithm results in three different outcomes:
1. We find a container with a matching plane and execute a focus change
2. We find a container with a matching plane, but there is no focus change,
   for example when we try to go left when we are focused on a leftmost
   element
3. We bubble up the container tree without ever finding a matching plane


### Navigation boundaries and navigation tree

The navigation tree is the entire container hierarchy, including all nodes (which
are always `Container`s) and leaves (`Navigable`s). For the previous example,
it looks like this:

![A diagram of the navigation tree for the tabbed menu example](https://user-images.githubusercontent.com/26321040/140542937-e28eed5e-70d5-4899-9c41-fb89b222469e.png)

Important: The navigation tree is linear: it doesn't have "dead end" branches,
it has as many nodes as there are depth levels.

The algorithm for finding the next focused element based on a `Command` is:
```python
def change_focus(
    focused: Navigable,
    cmd: Command,
    child_stack: List[ChildIndex],
    traversal_stack: List[Navigable],
) -> FocusResult:
  container = focused.parent
  if container is None:
    first_focused = traversal_stack.first() or focused
    return FocusResult.Uncaught(first_focused, cmd)

  next_focused = container.contained_focus_change(focused, cmd)
  if next_focused.is_caught:
    first_focused = traversal_stack.first() or focused
    return FocusResult.Caught(first_focused, container, cmd)

  elif next_focused.is_open:
    parent_sibling_focused = child_stack.pop()
    traversal_stack.push(focused)

    return change_focus(parent_sibling_focused, cmd, child_stack, traversal_stack)

  elif next_focused.is_sibling:
    first_focused = traversal_stack.first() or focused
    traversal_stack.remove_first()

    return FocusResult.FocusChanged(
      leaf_from= first_focused,
      leaf_to= next_focused.sibling,
      branch_from= traversal_stack,
    )
  else:
    print("This branch should be unreachable!")
```

## Design limitaions

### Unique hierarchy

If we implementation this with a cached navigation tree, it's going to be
easy to assume accidentally that we have a single fully reachable navigation
tree. This is not obvious, it's easy to just add the `Container` or `Navigable`
`Component` to an entity that doesn't have itself a `Container` parent more
than once.

I'm not sure how to handle this.

### No concurrent submenus

You have to "build up" reactively the submenus as you navigate through them,
since there can only be a single path through the hierarchy.

I'm implementing first to see how cumbersome this is, and maybe revise the
design with learning from that.

### Inflexible navigation

The developer might want to add arbitrary "jump" relations between various
menus and buttons. This is not possible with this design. Maybe we could add a
```rust
  bridges: HashMap<Command, Entity>,
```
field to the `Container` to reroute `Command`s that failed to change focus
within the menu. I think this might enable cycling references within the
hierarchy and break useful assumptions.

## Isn't this overkill?

Idk. My first design involved even more concepts, such as `Fence`s and `Bubble`
where each `Container` could be open or closed to specific types of `Command`s.
After formalizing my thought, I came up with the current design because it
solved issues with the previous ones, and on top of it, required way less
concepts.

For the use-case I considered, I think it's a perfect solution. It seems to
be a very useful solution to less complex use cases too. Is it overkill for
that? I think not, because we don't need to define relations at all, especially
for a menu exclusively in the physical plane. On top of that, I'm not creative
enough to imagine more complex use-cases.

[^1]: See [godot documentation](https://github.com/godotengine/godot-docs/blob/master/tutorials/ui/gui_navigation.rst)
[^2]: ie: includes children, grand-children, grand-grand-children etc.
