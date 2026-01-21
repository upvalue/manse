# manse

An experiment in project and application aware scrolling terminal emulation.
Make as many terminals as you want and scroll between them. Terminals are
automatically grouped into workspaces, based on the project you're in, and
plugins for apps/shells can update the UI to reflect what's going on in a given
terminal so you have better visibility.

# Caution: Vibe coded

This has been mostly vibe coded and may contain slop. This README is also slop,
albeit the kind of slop that was 100% written by a human. Aside from lacking
basic features that most terminal emulators have, it's also pretty hard on
battery usage at the moment. Mostly trying it out to see how it feels.

# Rambling

The basic theory of manse:

- Scrolling window management (PaperWM, niri, etc) is the right abstraction for
  managing lots of windows

- Terminal sessions don't need to be a black box to the emulator. Some
  information about what the active application is doing is really helpful in
  navigating and organizing terminal sessions.

So let's imagine the following scenario:

You have two projects open, at `$HOME/project1` and `$HOME/project2`. You have
three agent sessions open in the first and one in the second, and at least one
random terminal session open in both.

In a basic terminal emulator, this might look like having 5 tabs open, one
after the other. But when you try to scale this up, it's easy to get lost.
Things get shuffled around and terminals get noisy.

In manse, this splits up based on project. So you end up with two different
workspaces, one for project1 and one for project2. 

The "tabs" in this case can be renamed on the fly by you or running
applications. So you don't necessarily take the application title (though you
can).

But instead of having a horizontal bar of tabs that are named after whatever
the running application is, you have more of a tree:

```
- ~/project1
    - Agent: Implementing feature X
    - Agent: Implementing feature Y
    - Terminal: ~/project1
    - Agent: Fixing bug Z
- ~/project2
    - Agent: Implementing feature X
    - Neovim: editing src/feature-x.ts
```

# How does it work?

## Alacritty

All of the heavy lifting and any usability of this application can be
attributed to the fact that alacritty is exposed as a library. Thanks also to
[egui_term](https://github.com/Harzu/egui_term/) which wraps alacritty for use
in egui.

## Unix socket + env vars + .manse.json

Manse exposes a unix socket, so its CLI also serves as an interface for
terminal applications and others to inspect and modify its state.

`.manse.json` is currently a simple encoding of what manse project, if any, a
directory is attached to. (This allows us to auto-group e.g. multiple git
worktrees of the same project)

Information about the active manse workspace and instance are available in the
program environment; combined with the above this also allows programs to supply
some rich information about what's going on in them.


# Icons

Used without any particular permission. 
