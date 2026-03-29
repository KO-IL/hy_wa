#!/bin/bash

# Wall-Set GUI 启动脚本
# 功能：启动wall-set服务并用独立窗口打开GUI

URL="http://localhost:7878"
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
    elif command -v wall-set > /dev/null 2>&1; then
        candidate="$(command -v wall-set)"
    fi

    printf '%s\n' "$candidate"
}

WALL_SET_BIN="$(select_wall_set_bin)"

# 检查wall-set是否已经运行
if ! pgrep -x "wall-set" > /dev/null; then
    # 启动wall-set服务
    if [ -n "$WALL_SET_BIN" ] && [ -x "$WALL_SET_BIN" ]; then
        "$WALL_SET_BIN" &
    else
        echo "wall-set binary not found." >&2
        exit 1
    fi

    # 等待服务启动
    for i in {1..10}; do
        if curl -s "$URL" > /dev/null 2>&1; then
            break
        fi
        sleep 0.5
    done
fi

# 用chromium app模式打开
chromium --app="$URL" --class=wall-set --name=wall-set &
