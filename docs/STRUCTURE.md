# 项目结构

## 目录结构

```
src/
├── main.rs           # 程序入口
├── app/              # 应用核心模块
│   ├── config.rs     # 配置加载与保存
│   ├── constants.rs  # 常量定义
│   ├── instance.rs   # GUI 实例锁
│   ├── model.rs      # 数据模型
│   └── state_ops.rs  # 状态操作
├── engine/           # 壁纸引擎模块
│   └── wallpaper.rs  # 壁纸应用逻辑
├── fs/               # 文件系统模块
│   ├── paths.rs      # 路径解析
│   └── scanner.rs    # 壁纸扫描
├── gui/              # GUI 模块
│   ├── page.html     # Web 界面
│   └── web.rs        # HTTP 服务器
└── util/             # 工具模块
    └── text.rs       # 文本处理
```

## 模块说明

### main.rs
程序入口点，处理命令行参数：
- 无参数：启动 GUI 服务器
- `wall-set restore`：恢复上次壁纸
- `wall-set <path>`：应用指定壁纸

### app/

**config.rs** - 配置管理
- `load_settings()`：从 `~/.config/wall-set/settings.conf` 加载配置
- `save_settings()`：保存配置到文件

**constants.rs** - 常量定义
- `DEFAULT_ENGINE_BIN`：默认引擎二进制名称
- `DEFAULT_OUTPUT`：默认显示器输出
- `SERVER_ADDR`：GUI 服务器地址
- `CONFIG_RELATIVE_PATH`：配置文件路径
- 支持的媒体文件扩展名

**instance.rs** - 实例管理
- 使用文件锁防止多个 GUI 实例同时运行
- `acquire_gui_instance_lock()`：获取实例锁

**model.rs** - 数据结构
- `Settings`：用户配置（output, scan_root, last_wallpaper）
- `MediaKind`：媒体类型枚举（Image, Video, Project, Other）
- `WallpaperEntry`：壁纸条目（路径、名称、类型、缩略图）
- `AppState`：应用运行时状态

**state_ops.rs** - 状态操作
- `apply_and_save()`：应用壁纸并保存到配置
- `restore_last_wallpaper()`：恢复上次使用的壁纸

### engine/

**wallpaper.rs** - 壁纸引擎核心
- `apply_wallpaper()`：根据媒体类型选择合适的引擎
- `run_swww_image()`：使用 swww 显示图片壁纸
- `run_wallpaperengine_project()`：运行 Wallpaper Engine 项目
- `run_wallpaperengine()`：启动 Wallpaper Engine
- `stop_wallpaperengine()`：停止引擎进程
- `prepare_engine_launch()`：准备引擎启动参数（路径、工作目录、库路径）
- Wayland 环境配置
- 进程管理（启动、终止、检测）

### fs/

**paths.rs** - 路径处理
- `resolve_scan_root()`：解析扫描根目录

**scanner.rs** - 壁纸扫描
- `scan_wallpapers()`：递归扫描目录，查找支持的壁纸
- `classify_target()`：判断文件类型
- `resolve_project_dir()`：解析项目目录

### gui/

**web.rs** - HTTP 服务器
- `run_gui_server()`：启动 TCP 服务器
- `handle_connection()`：处理 HTTP 请求
- API 端点：
  - `GET /`：返回 HTML 页面
  - `GET /api/list`：获取壁纸列表和状态
  - `GET /api/refresh`：重新扫描目录
  - `GET /api/set_output`：设置显示器输出
  - `GET /api/set_root`：设置扫描根目录
  - `GET /api/apply`：应用壁纸
  - `GET /api/file`：获取媒体文件

**page.html** - Web 界面
- 暗黑主题 UI
- 壁纸网格展示
- 缩略图加载
- 工具栏（设置扫描根目录、输出显示器）

### util/

**text.rs** - 文本工具
- `url_decode()`：URL 解码
- `url_encode()`：URL 编码
- `json_escape()`：JSON 字符串转义

## 数据流

```
用户操作
    ↓
main.rs (解析参数)
    ↓
┌─────────────────────────────────────────┐
│  GUI 模式 (web.rs)                       │
│    ↓                                    │
│  HTML 页面 ←→ HTTP API                   │
│    ↓                                    │
│  state_ops.rs (应用壁纸)                 │
└─────────────────────────────────────────┘
    ↓
wallpaper.rs (根据类型选择引擎)
    ↓
┌──────────────────┬─────────────────────┐
│ swww (图片)      │ Wallpaper Engine    │
│                  │ (视频/项目)          │
└──────────────────┴─────────────────────┘
    ↓
config.rs (保存状态)
```
