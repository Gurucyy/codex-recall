# Codex Recall

[English](./README.md) | 中文

[![Rust](https://img.shields.io/badge/Rust-stable%2B-000000?logo=rust&logoColor=white)](./Cargo.toml)
[![Node.js](https://img.shields.io/badge/Node.js-20%2B-339933?logo=node.js&logoColor=white)](./package.json)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](./apps/desktop/src-tauri/tauri.conf.json)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

`codex-recall` 是一个非官方、本地优先的桌面应用，用于查看、搜索、
导出、备份、诊断，并谨慎修复本机 OpenAI Codex 会话数据。

它适用于这类场景：Codex Desktop 侧边栏、项目视图或搜索里看不到某些会话，但本地
磁盘上的底层会话文件可能仍然存在。

> [!IMPORTANT]
> 本项目不是 OpenAI 官方项目。默认 vault 工作流对原始 Codex 数据保持只读，不上传
> transcript。Repair Center 仍是实验功能，所有写入路径都必须本地执行、显式门控、
> 先备份，并尽可能可回滚。

## 目录

- [为什么需要它](#为什么需要它)
- [核心功能](#核心功能)
- [快速开始](#快速开始)
- [开发校验](#开发校验)
- [应用流程](#应用流程)
- [可选 CLI](#可选-cli)
- [安全与隐私](#安全与隐私)
- [Repair Center](#repair-center)
- [仓库结构](#仓库结构)
- [文档](#文档)
- [参与贡献](#参与贡献)
- [安全报告](#安全报告)
- [许可证](#许可证)

## 为什么需要它

Codex Desktop 会在本地保存多种会话证据：rollout JSONL、session index 记录、
state SQLite 文件以及 global state 元数据。当这些来源之间发生漂移、缺失或与侧边栏
不一致时，很难判断哪些会话仍然存在，以及哪些恢复动作是安全的。

本项目提供一个本地 recovery vault，用于查看这些证据，同时保证默认扫描、查看、搜索、
导出、备份和报告流程不会变成对原始 Codex 数据的写回操作。

## 核心功能

- 自动发现 Codex home，或手动选择 Codex home。
- 扫描 `sessions/` 和 `archived_sessions/`。
- 容错解析 rollout JSONL、`session_index.jsonl`、所有 `state_*.sqlite` 文件和
  `.codex-global-state.json`。
- 规范化本地会话证据，并按 workspace 分组。
- 按标题、内容、路径、thread ID、诊断、归档状态和风险搜索。
- 查看 transcript、元数据、原始证据摘要和诊断信息。
- 导出 Markdown、JSON 和自包含 HTML。
- 创建带 SHA-256 manifest 的完整 `.codex` 备份。
- 生成带脱敏选项的本地恢复报告。
- 通过受保护的 Repair Center 工作流执行选定修复。

## 快速开始

### 前置要求

- Rust stable 和 Cargo
- Node.js 20 或更新版本
- pnpm 10 或更新版本
- 当前操作系统所需的 Tauri 2 平台依赖

### 运行桌面应用

```bash
git clone https://github.com/Gurucyy/codex-recall.git
cd codex-recall
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` 会打开原生桌面窗口。本项目不是浏览器工具，不需要本地 HTTP 服务或
localhost 浏览器工作流。

### 给编码 Agent 的一句话

如果你使用 Codex、Claude Code、Cursor、Windsurf 或其他编码 Agent，可以直接给它这句
任务说明：

```text
如果需要，请帮我 clone codex-recall，安装依赖，运行最小必要校验，并根据 README.md 启动 Tauri 桌面应用。
```

## 开发校验

运行前端校验：

```bash
pnpm typecheck
pnpm build
```

运行 Rust 校验：

```bash
cargo test --workspace
```

构建桌面安装包：

```bash
pnpm tauri build
```

在 macOS 上会产出 `.app` 应用包和 `.dmg` 安装包；在 Windows 上会产出
NSIS `.exe` 和 MSI `.msi` 安装包。

如果要通过 GitHub 生成 Release 资产，推送版本 tag 即可：

```bash
git tag -a v0.1.0 -m "Release 0.1.0"
git push origin v0.1.0
```

`desktop-release` workflow 会构建 macOS 和 Windows 包，并把 `.dmg`、`.exe`
和 `.msi` 上传到对应 tag 的 GitHub Release。

## 应用流程

1. 打开原生桌面应用。
2. 自动发现或手动选择 Codex home。
3. 扫描本地 Codex 数据。
4. 浏览 active 和 archived sessions。
5. 按标题、内容、路径、thread ID、诊断、归档状态或风险搜索会话。
6. 打开会话，查看 transcript、元数据、证据和诊断。
7. 导出选定会话或生成恢复报告。
8. 在尝试任何 Repair Center 工作流之前，先创建完整备份。

## 可选 CLI

CLI 复用同一个本地 Rust core：

```bash
codex-vault discover
codex-vault scan --codex-home ~/.codex --out scan-report.json
codex-vault list --codex-home ~/.codex --risk high
codex-vault search --codex-home ~/.codex --q "diagnostic:R012 risk:high"
codex-vault export --codex-home ~/.codex --format markdown --out ./exports
codex-vault backup --codex-home ~/.codex --out ./codex-backup.zip
codex-vault report --codex-home ~/.codex --out recovery-report.md
```

没有 `serve` 命令。

## 安全与隐私

默认扫描、查看、搜索、导出、备份和报告工作流对原始 Codex 数据保持只读。

应用不会：

- 在普通 vault 工作流中写入原始 `state_*.sqlite`
- 修改 `session_index.jsonl`
- 修改 `.codex-global-state.json`
- 移动、重命名或删除 Codex session 文件
- 上传 transcripts、路径、元数据、诊断、报告或备份
- 要求用户打开浏览器或使用 localhost 工作流
- 在导出的 HTML 中加载远程 JavaScript、CSS、字体或图片

导出、备份、报告、rollback package 和应用设置是正常写入目标。导出、备份、报告和
rollback 输出只会写入用户选择的路径，或在适当情况下写入应用配置目录。

导出的文件和备份可能包含私有代码、transcript 内容、本地路径、命令和其他敏感数据。
分享前请先检查。

## Repair Center

Repair Center 是实验功能，并且必须受保护。它不是盲目的一键修复工具。

任何可写修复路径都需要选定 sessions、生成 repair plan、在复制出来的 `CODEX_HOME`
上 dry-run、真实 apply 前创建完整 `.codex` 备份、本地 patch 前创建 rollback package、
明确确认 Codex Desktop 已关闭，并在 apply 后执行验证扫描。

优先使用官方 `codex app-server --listen stdio://` 操作。只有官方路径无法覆盖时，才使用
窄范围本地 patch。目前本地 patch 范围仅限缺失 workspace-root hints，以及选定 rollout
JSONL 首行 `session_meta.payload.thread_source` 回填。

## 仓库结构

```text
apps/desktop/                 Tauri 2 + React 桌面应用
crates/codex-vault-core/      scanner、diagnostics、search、export、backup、repair core
crates/codex-vault-cli/       可选本地 CLI
docs/                         architecture、privacy 和 roadmap 文档
fixtures/                     仅用于 synthetic fixture 说明和测试数据
```

## 文档

- [Architecture](./docs/architecture.md)
- [Privacy](./docs/privacy.md)
- [Roadmap](./docs/roadmap.md)

## 参与贡献

欢迎贡献，但必须保留本地优先、桌面优先、默认只读的安全模型。请先阅读
[CONTRIBUTING.md](./CONTRIBUTING.md) 和 [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)。

测试只能使用 synthetic fixtures。请勿提交真实 Codex session 数据、transcripts、本地路径、
备份、报告、导出文件或 repair 输出。

## 安全报告

提交漏洞或分享诊断证据前，请阅读 [SECURITY.md](./SECURITY.md)。不要在公开 issue 中
发布私有 transcripts、tokens、本地路径或备份。

## 许可证

MIT。详见 [LICENSE](./LICENSE)。
