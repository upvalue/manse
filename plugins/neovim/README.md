# Manse Neovim Plugin

Updates Manse's terminal description with the currently edited file.

## Installation

### lazy.nvim

```lua
{
  dir = "~/path/to/manse-rs/plugins/neovim",
  config = function()
    require("manse").setup()
  end,
}
```

### Manual

Add the plugin directory to your runtimepath and call setup:

```lua
vim.opt.runtimepath:append("~/path/to/manse-rs/plugins/neovim")
require("manse").setup()
```

## Configuration

```lua
require("manse").setup({
  manse_cmd = "manse",  -- path to manse binary
})
```
