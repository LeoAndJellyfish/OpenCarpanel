# 桌面更新进度反馈设计

## 目标

应用内更新必须让用户立即知道操作已开始、当前处于哪个阶段，以及下载是否仍在推进。界面保持现有控制中心的简洁信息密度，不增加容易抖动的速度或剩余时间估算，也不改变签名验证和 Host 生命周期。

## 方案

更新流程沿用现有 Rust command，但为命令增加一个 Tauri IPC Channel。Rust updater 的分块回调累计真实下载字节，并依次发送 `preparing`、`downloading`、`verifying`、`installing`。前端不轮询，也不经远程服务转发状态。

```text
用户点击安装
    ↓
Preact 立即显示准备状态
    ↓
Rust updater.check
    ↓
download(chunk, content-length) ──IPC Channel──→ 百分比 / 字节进度
    ↓
签名验证 ──IPC Channel──→ 验证状态
    ↓
停止 Host、启动安装器 ──IPC Channel──→ 安装状态
```

下载总大小存在时显示真实百分比和“已下载 / 总大小”。服务器不提供总大小时，进度条进入不定状态，不伪造百分比。检查、准备和验签阶段同样使用不定状态。失败后保留最后一次有效进度，使用错误色标识并恢复重试入口。

## 视觉与可访问性

进度区域只包含当前阶段、百分比和可用的字节信息。确定进度通过 `transform: scaleX()` 更新；不定进度使用单个合成层平移。所有状态通过 `aria-live` 宣告，进度轨道使用 `progressbar` 语义。`prefers-reduced-motion` 下取消循环位移动画，改为静态部分填充。

## 安全边界

- updater 下载完成回调只表示开始验签；只有 `download().await` 成功返回后才进入安装状态。
- 下载或签名失败仍不停止 Host，也不执行安装器。
- Host 只在验签成功后停止；同步启动安装器失败时继续沿用原有恢复流程。
- 本次不修改版本号、Tag 或 Release。

## 验证

- TypeScript 单元测试覆盖乱序保护、百分比限制、未知总量和字节格式化。
- Rust 单元测试覆盖累计字节与 IPC JSON 契约。
- 运行桌面 TypeScript 检查、测试和构建，以及 Rust fmt、Clippy 和桌面 crate 测试。
