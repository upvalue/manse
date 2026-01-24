# manse.fish

# Only set up hook if running inside manse
set -q MANSE_SOCKET
or return

set -g MANSE_CMD manse

# Hook for manse workspace switching on directory change
function __manse_hook --on-variable PWD
    # Early exit if no .manse.json in current directory
    test -f .manse.json
    or return

    # Parse workspace name and invoke manse
    set -l workspace_name (command jq -r '.workspaceName // empty' .manse.json 2>/dev/null)
    test -n "$workspace_name"
    and command $MANSE_CMD term-to-workspace --socket $MANSE_SOCKET --workspace-name "$workspace_name"
end

# Run once on shell startup
__manse_hook
