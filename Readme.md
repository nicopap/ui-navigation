# Ui navigation

A generic UI navigation algorithm meant to be adaptable to any UI library, but
currently limiting itself to targeting the Bevy engine default UI library.

The design document is [available here](https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md).

## Structure

This repo contains two crates:
* `generic`: An implementation of the navigation algorithm using the classical
  tree data structure. It is here for reference and as an help to understand
  the algorithm for people not necessarilly familiar with ECS.
* `bevy`: A completely independent implementation of the same algorithm using
  the bevy ECS. It doesn't at all depend on `generic`.

## Roadmap

[X] Basic flat hierarchy 2D navigation (requires element location deduction)
[ ] Hierarchical navigation with Action/Cancel (requires tree layer without
    an active trail)
[ ] Tabbed navigation demo (requires Forward/Backward commands support)
[ ] Complex hierarchy with focus memory (requires tree)

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

