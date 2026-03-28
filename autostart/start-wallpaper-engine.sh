#!/usr/bin/env bash

set -eu

for _ in $(seq 1 30); do
    if /usr/bin/swww query >/dev/null 2>&1; then
        sleep 2
        /home/wang/hw/wall-set/target/release/wall-set restore
        exit 0
    fi

    /usr/bin/swww-daemon >/dev/null 2>&1 &
    sleep 1
done

exit 1
