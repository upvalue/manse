# manse

Manse is a terminal manager app

The basic theory of manse:

- Scrolling window management (PaperWM, niri, etc) is the right abstraction for
  managing lots of windows

- Terminal sessions don't need to be a black box to the application embedding
  them. Some information about what the active application is doing is really
  helpful in navigating and organizing.

- However, terminal is the right main interface for working on projects.
  Terminal apps are manifold and composable, UI apps are always too tied to
  specific workflows

So let's imagine the following scenario:

You have two projects open, at `$HOME/project1` and `$HOME/project2`. You have
three claude code sessions open in the first and one in the second, and at
least one random terminal session open in both.

In a basic terminal emulator, this might look like having 5 tabs open, one
after the other. But when you try to scale this up, it's a bit too easy to get
lost or shuffle them around.

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
    - Terminal: neovim editing src/feature-y.ts
    - Agent: Fixing bug Z
- ~/project2
    - Agent: Implementing feature X
    - Terminal: neovim editing src/feature-z.ts
```

# How does it work?

## Unix socket + env vars + .manse.json

Manse exposes a unix socket, so its CLI also serves as an interface for
terminal applications and others to inspect and modify its space.

`.manse.json` is currently a simple encoding of what manse project, if any, a
directory is attached to. (This allows us to auto-group e.g. multiple git
worktrees of the same project)

Information about the active manse workspace and instance are available in the
program environment; combined with the above this also allows programs to supply
some rich information about what's going on in them.

# Caution: Vibe coded

The application has been entirely vibe coded and may contain slop. This README
is also slop, albeit the kind of slop that was written by a human.
