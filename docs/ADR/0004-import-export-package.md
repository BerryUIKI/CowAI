# ADR 0004：导入/导出包格式

- **状态**：Accepted
- **日期**：2026-11-03
- **关联**：`docs/PRD.md` §7.1、`docs/ARCHITECTURE.md` §4.3

## 背景

本地模式与团队模式的数据相互独立，v1 通过导入/导出迁移。需要一种可移植、可审查、跨版本兼容的包格式，并避免把凭证与密码哈希带出。

## 决策

1. **扩展名 `.cowpkg`，ZIP 容器**。
2. **`manifest.json`** 放在根目录，包含：
   - `format`: `"cowai-package"`；
   - `schema_version`: 整数；
   - `kind`: `workspace` / `template` / `task-archive`；
   - `app_version`、`created_at`、`exported_by`；
   - `checksums`: 文件路径到 SHA-256 的映射。
3. **内容组织**：
   ```
   manifest.json
   workspace.json
   agents.json
   templates.json
   channels.jsonl
   messages.jsonl
   tasks.json
   runs.jsonl
   audit.jsonl
   attachments/...
   ```
   大量数据（消息、运行记录、审计）使用 JSONL 以便流式读写。
4. **不导出 secrets**：API Key、模型凭据、密码哈希、会话/API Token 一律排除；导入后需重新配置。
5. **v1 导入语义**：
   - `template`：合并到模板库（按 `id` + 版本去重）；
   - `workspace`：创建新工作区，不做原地合并与 ID 重映射；
   - `task-archive`：导入到指定频道/工作区作为只读归档。
6. **校验与事务**：导入前校验 manifest、`schema_version`、checksums 与包大小限制；失败整体回滚，不留半成品。
7. **兼容策略**：`schema_version` 单调递增；导入器支持当前版本，破坏性变更需提供迁移器；新增字段保持向后兼容。

## 理由

- JSON/JSONL 可读、可 diff、可审查，避免直接搬运 SQLite 文件带来的版本与平台耦合；
- ZIP 容器适合携带附件与多文件结构；
- 不导出 secrets 降低泄露风险；
- "只创建新工作区、不做合并"显著降低 v1 复杂度与冲突处理成本。

## 后果

- 需要实现流式 ZIP 读写与 checksum 校验；
- 导入后模型与工具凭证需要重新配置；
- 未来若支持合并/同步，需要额外的 ID 映射与冲突解决机制；
- 包格式需要版本化文档与兼容性测试。

## 备选方案

- **直接导出 SQLite 文件**：实现简单，但跨版本/跨平台不可靠，且包含敏感数据。
- **单一 JSON 文件**：附件与大数据场景不友好。
- **在线同步协议**：体验好，但 v1 复杂度高，且与"数据独立"的决策冲突。
