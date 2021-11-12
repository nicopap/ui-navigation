# Ui navigation

A generic UI navigation algorithm meant to be adaptable to any UI library, but
currently limiting itself to targeting the Bevy engine default UI library.

The design document is [available here](https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md).

### Examples

Check out the `examples` directory for bevy examples.

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
- [ ] Minor refactor of `resolve` function + Add FocusableButtonBundle to
      examples to simplify them greatly
- [ ] Replace most calls to `.iter().find(…)` for child non_inert by checking
      the `NavFence`'s `non_inert_child` rather than `query.nav_fences`. This
      fixes the most likely hotspot which is the recursive function
      `children_focusables`.
- [ ] Descend the hierarchy on Next and Previous (requires non_inert_child
      otherwise it's going to be very difficult to implement)
- [ ] Add mouse support
- [ ] Even more lööps (`North`/`South` and `East`/`West` locked)

# License

Copyright © 2021 Nicola Papale

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the "Software"),
to deal in the Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the
Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included
in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

