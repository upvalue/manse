# Claude Code Plugin for Manse

Hooks for integrating Claude Code with Manse terminal management.

## Hooks

### notify.sh (Stop hook)
Notifies the Manse terminal when Claude Code stops, showing an indicator until focused.

### clear-desc.sh (EndSession hook)
Clears the terminal description when a Claude Code session ends.

## Configuration

Add to your Claude Code settings (`~/.claude/settings.json`):

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/manse/plugins/claude/notify.sh"
          }
        ]
      }
    ],
    "EndSession": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/manse/plugins/claude/clear-desc.sh"
          }
        ]
      }
    ]
  }
}
```

Replace `/path/to/manse` with the actual path to your manse installation.

## Requirements

- The `manse` binary must be in your PATH
- Terminals must be spawned by Manse (so `MANSE_TERMINAL` env var is set)
