# wall-set

Linux 壁纸管理器，支持图片和 Wallpaper Engine 视频/项目。

## 功能

- Web GUI 界面浏览和切换壁纸
- 支持图片壁纸（通过 [swww](https://github.com/HikariLly/swww)）
- 支持 Wallpaper Engine 项目和视频
- 命令行直接应用壁纸
- 恢复上次使用的壁纸

## 依赖

- **swww** - Wayland 动态壁纸守护进程（用于图片）
- **Wallpaper Engine for Linux** ([Almamu](https://github.com/Almamu/linux-wallpaperengine)) - 用于视频和项目
- Rust 工具链

## 安装

```bash
cargo build --release
sudo cp target/release/wall-set /usr/local/bin/
```

## 使用

### GUI 模式

```bash
wall-set
```

访问 http://localhost:7878

### 命令行模式

应用壁纸：

```bash
wall-set /path/to/wallpaper/project.json
wall-set /path/to/image.jpg
wall-set /path/to/video.mp4
```

恢复上次壁纸：

```bash
wall-set restore
```

### 环境变量

- `LINUX_WALLPAPERENGINE_BIN` - Wallpaper Engine 可执行文件路径（默认：`linux-wallpaperengine`）
- `WALL_SET_ENGINE_DEBUG=1` - 调试模式（显示引擎输出）

## 配置

配置文件位于 `~/.config/wall-set/settings.conf`：

```
output=DP-3
root=/path/to/wallpapers
last=/path/to/last/wallpaper
```

## 自动启动

```bash
cp autostart/wall-set.desktop ~/.config/autostart/
```

## 支持的格式

- 图片：jpg, jpeg, png, bmp, gif, webp
- 视频：mp4, mkv, webm, mov, avi, m4v
- Wallpaper Engine 项目（包含 project.json 的目录）
