# Dashboard

PWA-ready 手机/iPad 客户端。驾驶视图和编辑视图必须分包，编辑器依赖不得进入驾驶页面的初始资源。

驾驶页从 canonical telemetry 的 `meta.gameId` 选择 presentation profile，只在游戏变化时更新 Preact 状态；速度、RPM 等高频值继续由单一 `requestAnimationFrame` 循环直接更新 DOM。`f1-24`、`f1-25`、`ets2`、`ats` 各自拥有独立 Host layout ID，F1 与卡车状态组件分别使用 DRS 和 SCS bridge 语义。未知 game ID 安全回退到 `default`，不自动创建任意布局。
