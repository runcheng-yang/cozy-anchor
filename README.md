# CozyAnchor

> P2P 多端同步工具 — 你的数据，你做主。

[![License](https://img.shields.io/badge/license-Commercial_Reserved-orange)](LICENSE)

## 简介

CozyAnchor 是一款基于 **P2P（点对点）网络** 的多端同步工具，支持消息、待办事项、文件在设备间直接传输，无需依赖中心服务器。你的数据只在你自己的设备之间流动，无需上传到第三方云端。

## 功能特性

| 功能 | 状态 | 说明 |
|------|------|------|
| 设备配对 | ✅ | 通过 NodeID 安全配对设备 |
| 消息传输 | ✅ | 设备间点对点发送消息 |
| 待办同步 | ✅ | 跨设备同步待办事项 |
| 文件传输 | ✅ | 点对点文件发送与接收 |
| 离线可用 | ✅ | 本地 SQLite 存储，无网也能用 |

## 技术架构

```
┌─────────────────────────────────────────┐
│              Frontend                   │
│         Vite + Vanilla JS               │
├─────────────────────────────────────────┤
│              Tauri v2                   │
│         ┌───────────────┐               │
│         │  Rust Backend │               │
│         │  • Commands   │               │
│         │  • SQLite DB  │               │
│         │  • iroh P2P   │               │
│         └───────────────┘               │
└─────────────────────────────────────────┘
```

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端 | Vite + Vanilla JS | 轻量、无框架依赖 |
| 桌面框架 | Tauri v2 | Rust 编写的高性能原生应用 |
| 网络层 | iroh | 现代 P2P 网络库，去中心化连接 |
| 存储层 | SQLite (rusqlite) | 本地持久化存储 |
| 运行时 | Tokio | 异步 Rust 运行时 |

## 快速开始

### 环境要求

- [Rust](https://rustup.rs/) (>= 1.77)
- [Node.js](https://nodejs.org/) (>= 18)
- [Tauri 系统依赖](https://tauri.app/start/prerequisites/)

### 安装运行

```bash
# 克隆仓库
git clone https://github.com/runcheng-yang/cozy-anchor.git
cd cozy-anchor

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

### 使用步骤

1. **启动应用** — 每个设备会获得唯一的 NodeID
2. **添加设备** — 在"设备"页输入对方 NodeID 进行配对
3. **同步数据** — 选择已配对设备，同步消息和待办
4. **传输文件** — 选择目标设备，发送文件到对方

## 项目结构

```
cozy-anchor/
├── src/                    # 前端源码
│   ├── main.js            # 主入口 & UI 逻辑
│   └── style.css          # 样式
├── src-tauri/             # Tauri / Rust 后端
│   ├── src/
│   │   ├── main.rs        # 应用入口 & 连接处理
│   │   ├── commands.rs    # 前端调用的命令
│   │   ├── db.rs          # SQLite 数据库操作
│   │   ├── network.rs     # iroh P2P 网络
│   │   ├── protocol.rs    # 通信协议定义
│   │   ├── sync.rs        # 同步逻辑
│   │   └── models.rs      # 数据模型
│   ├── Cargo.toml         # Rust 依赖
│   └── tauri.conf.json    # Tauri 配置
├── package.json           # Node 依赖
└── vite.config.js         # Vite 配置
```

## 同步协议

CozyAnchor 使用基于时间戳的增量同步策略：

1. 设备 A 发送上次同步时间戳 `last_sync`
2. 设备 B 返回 `last_sync` 之后的数据
3. 设备 A 合并对方数据后发送自己的增量数据
4. 双方更新同步时间戳

冲突解决：以 **更新时间较晚** 的数据为准。

## 数据安全

- 所有数据存储在本地 SQLite 数据库 (`~/.cozy-anchor/`)
- P2P 通信通过 iroh 协议加密传输
- 无中央服务器，数据不经过第三方
- 设备间需要显式配对才能通信

## 路线图

- [ ] Android 端支持
- [ ] 自动发现局域网设备
- [ ] 端到端加密通信
- [ ] 历史版本回溯
- [ ] 插件扩展机制

## 许可证

本项目采用 [商业保留许可证](LICENSE)（Commercial Reserved License）。

- 开源可见，欢迎学习、研究、个人使用
- 商业使用需获得作者书面授权
- 版权归作者所有，作者保留后续商业化的权利

---

Made with by [runcheng-yang](https://github.com/runcheng-yang)
