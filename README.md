# Simple Bevy Game

使用 Bevy v0.17.2 开发的简单游戏示例。主要是设置了测试框架，确保 Github Copilot 能自己通过 e2e 测试验证改动，自己 TDD。

## 功能

- 窗口系统
- UI 系统
- 带阴影效果的按钮
- 点击交互
- 日志记录到 `logs/` 文件夹

## 运行

```bash
cargo run
```

## 测试

```bash
cargo test --test cucumber
```

测试框架通过 websocket 和消息队列与游戏进程通信，类似 Chrome Devtool 协议，以模仿 Playwright 测试 Electron 的效果。

## 日志

日志文件会自动保存在 `logs/game.log`，记录按钮点击位置等信息，以及某些步骤的屏幕截图。

## CI

Github Actions 里仿照 Bevy 官方的 <https://github.com/bevyengine/bevy/blob/main/.github/workflows/example-run.yml> 工作流设置可视化测试所需的依赖。

CI 里初次编译需要较长时间，后续如果只修改应用代码而没有更新依赖，缓存会命中，加载700多兆字节的缓存后，应该只需十几秒就能运行完毕。
