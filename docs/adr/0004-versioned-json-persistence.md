# ADR-0004：MVP 使用版本化 JSON 持久化

## Status

Accepted

## Context

MVP 需要保存设置、已配对设备、布局和主题。这些数据规模小、写入频率低，并且用户可能希望导入、导出或手工检查面板。引入 SQLite 会增加 schema、迁移和备份层，同时并没有复杂查询或事务关系需求。

## Decision

使用带 `schemaVersion` 和 `revision` 的 JSON 文档。每次保存先校验，再通过同目录临时文件、同步和原子替换写入；保留有限的 last-known-good 备份。所有旧版本读取通过显式迁移函数升级。布局更新使用乐观并发控制，revision 冲突不静默覆盖。

## Consequences

### Positive

- 数据透明、可移植，导入导出无需额外格式转换。
- 无数据库进程、连接池或迁移工具开销。
- 单个损坏布局可以隔离，不影响其他配置。
- 单元测试可以直接覆盖每个 schema 版本。

### Negative

- 跨多个文件的原子事务不自然。
- 大规模布局市场、全文搜索或历史版本查询不适合该方案。
- 必须自行正确实现原子替换、备份和迁移。

### Neutral

- 如果未来出现真实关系查询和事务需求，可以通过 config repository 接口迁移到 SQLite；MVP 不为此预留数据库抽象的全部复杂度。

## Alternatives Considered

- **SQLite：** 稳定可靠，但当前没有查询和关系需求。
- **仅浏览器 localStorage/IndexedDB：** 无法让多设备共享 Host 配置，清理浏览器数据会丢失布局。
- **云端配置：** 与本地运行和隐私目标冲突。

## References

- [主架构设计：配置与持久化](../plans/2026-08-11-opensimdash-architecture-design.md#10-配置与持久化)
