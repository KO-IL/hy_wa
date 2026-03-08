#!/usr/bin/env bash

set -eu

for _ in $(seq 1 30); do
    if /usr/bin/swww query >/dev/null 2>&1; then
        sleep 2
        /home/wang/hw/wall-set/target/release/wall-set &
        sleep 1
        cd /opt/linux-wallpaperengine
        export XDG_SESSION_TYPE=wayland
        export SDL_VIDEODRIVER=wayland
        export LD_LIBRARY_PATH=/opt/linux-wallpaperengine:/opt/linux-wallpaperengine/lib
        exec /opt/linux-wallpaperengine/linux-wallpaperengine \
            --screen-root DP-3 \
            --bg /home/wang/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/workshop/content/431960/3516294127 \
            --fps 60
    fi

    /usr/bin/swww-daemon >/dev/null 2>&1 &
    sleep 1
done

exit 1
