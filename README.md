# 斗地主 Rust + Tauri 项目

## 目标
- Rust 后端 + 前端 UI
- WebSocket 长连接通信
- 房间管理（随机用户 ID，可加入已有房间）
- 支持 Web / Windows / Android 打包（Tauri + Web 前端）

## 架构
- `game-core`：纯 Rust 规则与回合状态机（可测试）
- `server`：WebSocket 服务端 + 房间管理
- `ui`：前端 Web UI（浏览器 / Tauri WebView）
- `tauri-app`：Tauri 包装，加载 `ui` 作为前端

## 当前规则覆盖
- 单张、对子、三张、三带一、三带二
- 顺子、连对、飞机（不带翼）
- 炸弹、王炸、四带二

> 叫地主与计分已简化：开局随机地主，未实现抢地主/倍数体系。

## 运行
### 安装前端依赖
```bash
cd ui
npm install
npm run e2e:install
```

### 后端
```bash
cargo run -p server
```
或使用脚本（会先清理旧的 `server.exe` 进程再启动）：
```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-server.ps1
# 可选：指定端口（用于启动前占用检查）
powershell -ExecutionPolicy Bypass -File scripts/start-server.ps1 -Port 33030
# 可选：release 构建启动
powershell -ExecutionPolicy Bypass -File scripts/start-server.ps1 -Release
```
后端会同时提供本地卡牌资源：`http://<server-host>:33030/assets/cards/*.png`。

### Web 前端
使用任意静态服务器打开 `ui/index.html`，例如：
```bash
cd ui
python -m http.server 5173
```
浏览器打开 `http://127.0.0.1:5173`，保持 WebSocket 连接到 `ws://100.70.102.165:33030/ws`。
Web 端卡牌图片会从后端 `/assets/cards` 获取，不依赖外部 CDN。

### Tauri (Windows / Android)
```bash
cd tauri-app/src-tauri
cargo tauri dev
```

### Android 打包脚本
```powershell
# 仅检查环境与参数（不执行构建）
powershell -ExecutionPolicy Bypass -File scripts/package.ps1 -Target android -DryRun

# Debug APK（可直接安装）
powershell -ExecutionPolicy Bypass -File scripts/package.ps1 -Target android -Configuration debug

# Release APK（自动签名并验签，默认使用 ~/.android/debug.keystore 便于真机侧载）
powershell -ExecutionPolicy Bypass -File scripts/package.ps1 -Target android -Configuration release

# Release APK（使用自定义 keystore）
powershell -ExecutionPolicy Bypass -File scripts/package.ps1 -Target android -Configuration release `
  -KeystorePath "D:\keys\release.keystore" `
  -KeystoreAlias "release" `
  -KeystorePassword "your_store_password" `
  -KeyPassword "your_key_password"

# 按 ABI 分包
powershell -ExecutionPolicy Bypass -File scripts/package.ps1 -Target android -Configuration release -SplitPerAbi
```

## 测试
### Rust 核心与服务端
```bash
cargo test -p game-core
cargo test -p server
```

### 前端 UI
```bash
cd ui
npm run test:unit
npm run test:e2e
npm run test:all
```

## 通信协议
客户端发送 JSON：
- `CreateRoom`
- `JoinRoom { room_id }`
- `Play { cards: ["S3", "H4", "BJ"] }`
- `Pass`

服务端返回：
- `Welcome { user_id, user_name }`
- `RoomCreated { room_id }`
- `Joined { room_id, you, you_name, player_count, started }`
- `RoomsList { rooms }`
- `RoomState { ... }`
- `PlayRejected { reason }`
- `GameOver { room_id, winner_id }`
- `GameRestarted { room_id }`

## 目录
- `game-core/` 规则与状态机
- `server/` WebSocket 服务端
- `ui/` 前端
- `tauri-app/` Tauri 壳
