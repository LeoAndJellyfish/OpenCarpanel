# ADR-0005：MVP 先交付本地 HTTP Web App

## Status

Accepted

## Context

手机通过局域网 IP 访问电脑 Host。普通浏览器可以加载 HTTP 页面并使用 WebSocket，但 Service Worker 与标准 PWA 安装流程通常需要 HTTPS 安全上下文；远端 HTTPS 页面再访问本地 HTTP/WebSocket又会遇到 mixed content 和额外平台限制。为每个 Host 配置受信任本地证书会显著增加首次连接复杂度。

## Decision

MVP 在用户选择的私有 LAN 接口提供 HTTP Web App，并保留 manifest、响应式布局和 standalone 元数据，使代码保持 PWA-ready。非安全上下文不注册 Service Worker，也不把“可安装”作为 MVP 验收条件。二维码短期令牌、设备会话、Origin/Host 校验、CSP 和私网警告降低本地风险。

完整 PWA 安装、本地 HTTPS 或原生壳作为独立后续决策，不混入实时遥测主链路。

## Consequences

### Positive

- 扫码即可连接，不要求用户安装 CA 或原生应用。
- Host 与浏览器完全本地运行，断网不影响已加载页面。
- MVP 可以集中验证游戏适配和延迟。

### Negative

- 不保证标准安装提示、Service Worker 缓存和部分安全上下文 API。
- 公共 Wi-Fi 上存在同网段窃听或篡改风险，必须明确提示。
- 页面刷新仍依赖 Host 在线提供资源。

### Neutral

- “PWA-ready”描述代码形态，不等于首版在所有浏览器上都可安装。

## Alternatives Considered

- **自签名本地 HTTPS：** 需要各设备信任证书，首次体验差。
- **远端托管 PWA + 本地 WebSocket：** 引入远程依赖和 mixed content 问题。
- **首版原生应用：** 偏离已确认的浏览器优先范围并增加发布维护成本。

## References

- [MDN Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [web.dev PWA install criteria](https://web.dev/articles/install-criteria)
