# Porthole

独立的桌面 SSH 终端。名字来自船上的**舷窗**：一扇看向远程机器的窗口。

当前仓库是 **Tauri 2** 空骨架，**没有** xterm、SSH、PTY 或命令转发实现。

## 技术栈

| 部分          | 技术                                | 目录         |
| ------------- | ----------------------------------- | ------------ |
| 前端          | Next.js + Hero UI + Tailwind CSS v4 | `src/`       |
| 桌面壳 / 后端 | Tauri 2（Rust）                     | `src-tauri/` |

独立的 Actix HTTP 服务已去掉。后续 PTY / SSH 会写在 `src-tauri` 里，通过 Tauri IPC 和前端通信。

## 目录

```text
Porthole/
├── src/          # 空的 Next.js 页面
└── src-tauri/    # 空的 Tauri 进程
```

## 运行空骨架

```bash
pnpm install
pnpm tauri dev
```

会先起 Next.js（`http://localhost:3000`），再打开 Porthole 窗口。

## 后端实现顺序（后续逐步带你做）

1. Tauri command
2. 事件通道（替代 WebSocket）
3. 本机 PTY + shell
4. SSH channel 连远程机器
5. 把远端输出推回前端
