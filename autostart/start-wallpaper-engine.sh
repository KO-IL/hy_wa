#!/usr/bin/env bash

set -eu

REPO_ROOT="/home/wang/hw/wall-set"
RELEASE_BIN="$REPO_ROOT/target/release/wall-set"
DEBUG_BIN="$REPO_ROOT/target/debug/wall-set"

select_wall_set_bin() {
    local candidate=""

    if [ -x "$DEBUG_BIN" ] && [ -x "$RELEASE_BIN" ]; then
        if [ "$DEBUG_BIN" -nt "$RELEASE_BIN" ]; then
            candidate="$DEBUG_BIN"
        else
            candidate="$RELEASE_BIN"
        fi
    elif [ -x "$DEBUG_BIN" ]; then
        candidate="$DEBUG_BIN"
    elif [ -x "$RELEASE_BIN" ]; then
        candidate="$RELEASE_BIN"
    elif command -v wall-set >/dev/null 2>&1; then
        candidate="$(command -v wall-set)"
    fi

    printf '%s\n' "$candidate"
}

WALL_SET_BIN="$(select_wall_set_bin)"

if [ -z "$WALL_SET_BIN" ] || [ ! -x "$WALL_SET_BIN" ]; then
    echo "wall-set binary not found." >&2
    exit 1
fi

for _ in $(seq 1 30); do
    if /usr/bin/swww query >/dev/null 2>&1; then
        sleep 2
        "$WALL_SET_BIN" restore
        "$WALL_SET_BIN" &
        exit 0
    fi

    /usr/bin/swww-daemon >/dev/null 2>&1 &
    sleep 1
done

exit 1
