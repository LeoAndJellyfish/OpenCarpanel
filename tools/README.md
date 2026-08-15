# Development tools

后续脚本放置位置：协议代码生成、UDP fixture 检查、遥测回放、性能报告和发布打包。脚本必须支持 Windows PowerShell；可移植部分优先使用 Rust。

## Codex 桌面快捷操作

本机快捷操作定义在 `.codex/environments/environment.toml`；`.codex/` 已被 Git 忽略，不随仓库分发：

- `发布新版本`：交互式读取 SemVer，二次确认后启动 Codex CLI 发布会话；包装脚本本身不直接提交、推送或创建 tag。
- `打开本地 Dev 预览`：运行 `npm run dev:desktop`。
- `打开 GitHub 项目`：运行 `gh repo view --web`，由 GitHub CLI 根据当前 remote 打开仓库。

发布包装脚本可以无副作用地检查：

```powershell
node tools/codex-actions/release.mjs --dry-run 0.4.0
node --test tools/codex-actions/release.test.mjs
```
