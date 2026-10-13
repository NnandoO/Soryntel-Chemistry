#!/usr/bin/env bash
set -euo pipefail
# Run under xvfb-run. Capture our own process, never the user's desktop.
task_binary="$(realpath "${1:-build/cma_chemistry}")"
task_output="$(realpath -m "${2:-build/ui-verification}")"
mkdir -p "$task_output"
task_pid=""
trap 'if [[ -n "$task_pid" ]]; then kill "$task_pid" 2>/dev/null || true; fi' EXIT
for task_panel in water aqueous mineral catalog; do
    "$task_binary" "--review-panel=$task_panel" > "$task_output/$task_panel.log" 2>&1 &
    task_pid=$!
    task_window=""
    for ((task_attempt=0; task_attempt<100; task_attempt++)); do
        task_window="$(xdotool search --onlyvisible --pid "$task_pid" --name 'CMA Chemistry' 2>/dev/null | head -1 || true)"
        [[ -n "$task_window" ]] && break
        kill -0 "$task_pid"
        sleep 0.1
    done
    [[ -n "$task_window" ]]
    sleep 1
    import -window "$task_window" "$task_output/$task_panel.png"
    kill "$task_pid"
    wait "$task_pid" || true
    task_pid=""
done
