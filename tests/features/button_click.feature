# language: zh-CN
功能: 按钮点击测试
  作为一个测试工程师
  我想要测试按钮点击功能
  以确保游戏交互正常工作

  场景: 点击主按钮
    假设 游戏已启动
    当 点击按钮 "main-button"
    那么 日志中应该包含 "test-id-button-clicked: main-button"
