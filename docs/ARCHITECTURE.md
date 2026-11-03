# CowAI 功能与技术架构（草案）

> **状态**：Draft v0.1（待与用户逐节确认）
> **日期**：2026-11-03
> **关联**：`docs/PRD.md`、`docs/ADR/0001-runtime-and-deployment.md`

---

## 1. 架构原则

1. **单一核心，多形态运行**：桌面端与服务端共享同一套 Rust 领域模型与业务逻辑。
2. **本地优先，多用户就绪**：默认无服务端可用，但账号、工作区与权限模型从 v1 就存在。
3. **API 优先**：桌面端、CLI 与未来集成统一通过 `cow-client` / HTTP + WebSocket 访问能力。
4. **事件驱动**：消息、任务状态、工具调用、审批都通过事件总线广播，便于实时 UI 与审计。
5. **配置优先于硬编码**：角色、工具、模板、审批策略均可配置，不把软件工程场景写死。
6. **安全默认**：危险操作默认需要审批；沙箱限制工作目录、超时与输出。
7. **小步演进**：每个阶段可独立发布、可测试、可回滚。

---

## 2. 运行时与部署

```
┌──────────────────────────── Desktop (Tauri) ────────────────────────────┐
│  Web 前端（频道 / 任务 / Agent / 审批 / 设置）                          │
│        │ Tauri IPC / HTTP                                               │
│  cow-client ──┬── 本地模式：进程内调用 cow-* 核心 + SQLite               │
│               └── 团队模式：HTTP/WebSocket → cow-server                  │
└─────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────── 自托管服务器 ──────────────────────────────┐
│  cow-server（HTTP + WebSocket API、认证、工作区、权限）                 │
│        │                                                                │
│  cow-agent / cow-tools / cow-llm / cow-bus / cow-store                  │
│        │                                                                │
│  SQLite（默认）或 PostgreSQL（P1）                                      │
└─────────────────────────────────────────────────────────────────────────┘

CLI（cowai）── cow-client ── 本地模式或远程服务器
```

- 桌面端默认本地模式，无需服务端；切换到团队模式只需更换 `cow-client` 的实现与连接配置。
- 服务端从 v1 起支持多用户；本地模式自动创建单用户工作区，但数据模型与服务端一致。

---

## 3. 功能域划分

### 3.1 身份与工作区（Identity & Workspace）

- 实体：`User`、`Workspace`、`Membership`、`Role`（Owner / Admin / Member）。
- 本地模式：首次启动自动创建本地 Owner 与默认工作区。
- 团队模式：账号注册/登录、工作区邀请、成员角色管理、API Token。
- v1 认证：本地账号密码 + API Token；OIDC/SSO 为 P1。

### 3.2 协作空间（Collaboration）

- 实体：`Channel`、`Thread`、`Message`、`Mention`、`TaskCard`、`ApprovalCard`。
- 人类与 Agent 都是频道成员；消息持久化并按时间线展示。
- `@Agent` 或任务卡片触发任务；Agent 的思考、工具调用、审批与交付物回写频道。
- 频道是协作主界面，但不限制使用方式：用户可自由创建频道与线程。

### 3.3 Agent 与模板（Agents & Templates）

- `AgentProfile`：名称、角色、目标、背景、模型、工具集、迭代上限、是否允许委派。
- 内置模板库：角色模板与团队模板（如开发团队、研究团队、写作团队），可在创建时选用。
- 模板支持自定义、导入/导出；模板只是起点，不绑定具体场景。

### 3.4 任务与编排（Tasks & Orchestration）

- 任务生命周期：`Pending → Assigned → Running → WaitingApproval → Completed / Failed / Cancelled`。
- v1 范围（已确认）：单 Agent 任务 + 顺序 Crew（`ProcessType::Sequential`）。
- 后续：层级编排（Manager Agent 委派）与 DAG 工作流。
- 任务支持前置依赖（`context_tasks`），前置交付物注入下游上下文。

### 3.5 工具与沙箱（Tools & Sandbox）

- `Tool` trait 与 `ToolRegistry`：工具名、描述、参数 Schema、异步执行。
- 内置工具：代码执行、文件读取、文件写入；后续扩展网络访问、检索、浏览器等。
- `SandboxBackend`：v1 为本地子进程；P1 增加 Docker；P2 评估 WASM。
- 资源限制：超时、输出截断、工作目录约束、环境变量过滤。

### 3.6 模型网关（Model Gateway）

- `LlmProvider` 抽象；v1 提供 OpenAI 兼容客户端（云 API、DeepSeek、Ollama、LocalAI）与 Mock。
- 模型配置属于工作区/Agent 级；支持连接测试。
- P1 增加流式响应、重试/退避、限流、Token 与成本统计。

### 3.7 记忆与知识（Memory）

- v1（已确认）：会话上下文 + 频道/任务历史作为短期记忆来源。
- P1：项目级长期记忆与知识库（检索增强），需要明确存储与隐私边界。
- Agent 记忆不得跨工作区泄漏。

### 3.8 持久化与审计（Persistence & Audit）

- `CowStore` 抽象；v1 实现 SQLite，P1 可选 PostgreSQL。
- 存储实体：用户、工作区、成员、频道、消息、Agent、模板、任务、运行记录、工具调用、审批、审计事件。
- 审计事件记录：谁在何时批准/拒绝了什么操作、Agent 执行了哪些工具。

### 3.9 接入层（Access）

- **桌面 UI**：Tauri Web 前端，频道、任务、Agent、审批、设置。
- **HTTP/WebSocket API**：`cow-server` 提供给桌面团队模式、CLI 与未来浏览器入口。
- **CLI**：`cowai` 用于初始化、诊断、无头运行与调试。
- **Webhook / IM**（P1）：Mattermost、Slack、飞书等。

---

## 4. 核心流程

### 4.1 从频道到交付

1. 用户在频道中 `@Agent`、创建任务卡片，或选择团队模板发起任务；
2. 编排器创建任务并分配 Agent（v1 顺序执行）；
3. Agent 调用 LLM，产生文本或工具调用请求；
4. 审批策略判定：自动放行或生成审批卡片；
5. 工具在沙箱中执行，结果回写频道与任务上下文；
6. 任务完成，交付物、执行记录与审计事件落库。

### 4.2 审批策略（多档）

参考 Claude Code 的多档权限模式：

| 模式 | 行为 |
| --- | --- |
| **Ask（默认）** | 每个敏感工具调用都请求批准；可批准一次、本会话、始终 |
| **SafeAuto** | 只读/安全操作自动放行，写入/执行仍请求批准 |
| **PlanOnly** | Agent 只做规划与只读调查，不执行写操作 |
| **Trusted** | 可信工作区内自动放行（需管理员开启），保留审计 |

- 规则优先级：工作区策略 > Agent 策略 > 全局默认。
- 细粒度规则：按工具、命令前缀、路径白名单/黑名单。
- 风险判定可组合静态规则与 LLM 分类器（待定）。

### 4.3 本地模式与团队模式

- **本地模式**：单用户、进程内核心、SQLite；无需网络与服务端。
- **团队模式**：多用户、远程 `cow-server`、共享工作区；桌面端只做展示与交互。
- 两种模式共用领域模型与 UI；差异封装在 `cow-client` 与认证层。
- **数据打通（v1）**：两种模式的数据相互独立，通过导入/导出迁移（工作区包、模板包、任务归档），不做双向同步。

---

## 5. 逻辑数据模型（草案）

| 实体 | 关键字段 |
| --- | --- |
| `User` | id、name、email、password_hash、created_at |
| `Workspace` | id、name、created_at |
| `Membership` | user_id、workspace_id、role |
| `Channel` | id、workspace_id、name、topic、created_at |
| `Message` | id、channel_id、sender、content、thread_id、created_at |
| `AgentProfile` | id、workspace_id、name、role、goal、backstory、model、tools、limits |
| `Template` | id、kind（agent/team）、name、payload |
| `Task` | id、workspace_id、channel_id、description、expected_output、assignee、status、context_tasks |
| `Run` | id、task_id、agent_id、status、started_at、finished_at |
| `ToolCall` | id、run_id、tool_name、arguments、result、approved_by |
| `Approval` | id、task_id、tool_call_id、status、decided_by、decided_at |
| `AuditEvent` | id、workspace_id、actor、action、target、created_at |

> 最终字段以 `cow-store` 的迁移脚本为准。

---

## 6. Crate 映射与规划

| Crate | 职责 | 状态 |
| --- | --- | --- |
| `cow-core` | 领域模型、错误、事件、Tool trait | 已有 |
| `cow-bus` | 事件总线与订阅 | 已有 |
| `cow-tools` | 工具注册与沙箱后端 | 已有（subprocess） |
| `cow-llm` | 模型 Provider 与类型 | 已有（OpenAI 兼容 + Mock） |
| `cow-agent` | Agent 运行循环与 Crew 编排 | 已有（Sequential） |
| `cow-cli` | `cowai` CLI | 已有 |
| `cow-store` | 持久化抽象与 SQLite/Postgres 实现 | 规划 |
| `cow-server` | HTTP/WebSocket API、认证、多用户 | 规划 |
| `cow-client` | 本地/远程统一客户端抽象 | 规划 |
| `cow-desktop` | Tauri 桌面壳与前端集成 | 规划 |
| 前端应用 | React + TypeScript（频道、任务、审批、Agent 管理 UI） | 规划 |

---

## 7. 权限与审批模型

- **工作区角色**：Owner（全部权限）、Admin（成员/Agent/策略管理）、Member（使用频道与任务）。
- **Agent 权限**：可使用的工具集、可用模型、是否允许委派、最大迭代次数。
- **审批决策**：记录审批人、时间、模式、规则依据，进入审计日志。
- **敏感操作**：文件写入/删除、命令执行、网络访问、跨工作区访问。

---

## 8. 非功能需求

- **安全**：默认拒绝危险操作；凭证加密存储；最小权限；审计可追溯。
- **性能**：本地模式冷启动 < 2s；频道消息实时推送；任务事件低延迟。
- **可靠性**：任务崩溃可恢复；SQLite 写入事务化；服务端可水平扩展（P1）。
- **可扩展**：工具、模型、模板、接入方式均通过接口扩展。
- **可观测**：结构化日志、任务/运行追踪、成本统计（P1）。
- **国际化**：UI 文案可本地化（至少中英文）。

---

## 9. 已确认决策与待确认问题

### 9.1 已确认（2026-11-03）

1. **编排范围**：v1 为单 Agent + 顺序 Crew；层级 Manager 与 DAG 放 P1。
2. **记忆范围**：v1 为会话 + 频道/任务历史；项目级长期记忆放 P1。
3. **前端技术**：React + TypeScript（Tauri WebView）。
4. **本地/团队模式**：数据独立，导入/导出为主，不做双向同步。

### 9.2 待确认

1. 模板格式与分发方式（内置 JSON、文件导入、在线仓库）。
2. 风险操作判定：纯静态规则、LLM 分类器，还是组合。
3. 导入/导出包的具体格式与兼容策略。
4. 认证细节与 OIDC/SSO 的优先级。
5. PostgreSQL 支持的时间点。
