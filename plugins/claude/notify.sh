#!/bin/sh

# Notify Manse terminal when Claude Code stops
# Only runs if MANSE_TERMINAL is set and manse command exists

if [ -n "$MANSE_TERMINAL" ] && command -v manse >/dev/null 2>&1; then
    manse term-notify
fi
