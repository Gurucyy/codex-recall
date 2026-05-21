import { useEffect, useMemo, useState } from "react";
import type { Diagnostic, VisibilityRisk, MessageRole, WorkspaceGroup } from "./types";

export type Language = "zh" | "en";

const storageKey = "codex-recall.language";

const dictionary = {
  zh: {
    appName: "Codex Recall",
    readOnlyBadge: "只读模式",
    homePlaceholder: "选择 Codex 数据目录",
    manualHome: "手动",
    chooseHome: "选择目录",
    actions: "操作",
    clearSearch: "清空搜索",
    collapseSidebar: "收起会话列表",
    expandSidebar: "展开会话列表",
    shownInCodex: "Codex 中显示",
    archivedInCodex: "Codex 已归档",
    localOnly: "仅本地存在",
    metadataOnly: "元数据记录",
    scan: "扫描",
    export: "导出",
    backup: "备份",
    report: "报告",
    repair: "修复中心",
    restoreToCodex: "恢复到 Codex",
    restoreToCodexHint: "为这个仅本地存在的会话生成安全恢复计划",
    language: "语言",
    chinese: "中文",
    english: "EN",
    scanStarted: "开始扫描本地 Codex 数据",
    scanFinished: "扫描完成",
    scanStageStart: "准备扫描",
    scanStageSnapshot: "创建只读快照",
    scanStageJsonl: "读取转录文件",
    scanStageIndex: "读取会话索引",
    scanStageSqlite: "读取 SQLite 状态",
    scanStageGlobalState: "读取全局状态",
    scanStageNormalize: "归一化会话",
    scanStageDiagnostics: "应用诊断规则",
    scanStageSummary: "整理扫描摘要",
    scanStageProcessing: "整理扫描结果",
    scanComplete: "扫描完成：{count} 个会话",
    exportDone: "已导出到 {path}",
    backupDone: "备份已保存到 {path}",
    reportDone: "恢复报告已保存到 {path}",
    repairPlanReady: "修复计划已生成：{count} 个操作",
    dryRunDone: "Dry-run 完成：{status}",
    applyDone: "修复执行完成：{status}",
    rollbackDone: "回滚完成：{count} 个文件",
    sessions: "会话",
    active: "未归档",
    archived: "已归档",
    highRisk: "高风险",
    mediumRisk: "中风险",
    transcriptFiles: "转录文件",
    databaseRows: "数据库记录",
    workspaces: "工作区",
    searchPlaceholder: "搜索标题、正文、路径、线程 ID，或输入 diagnostic:R012 / risk:medium",
    noTimestamp: "无时间",
    unknownWorkspace: "未识别工作区",
    selected: "当前会话",
    all: "全部会话",
    exportPanel: "导出",
    exportSelected: "导出当前会话",
    exportAll: "导出全部",
    redact: "脱敏路径和疑似密钥",
    backupReport: "备份与报告",
    createBackup: "创建 zip 备份",
    generateReport: "生成恢复报告",
    noSessionTitle: "未选择会话",
    noSessionBody: "从左侧选择一个会话查看转录、元数据和诊断。",
    emptyTitle: "选择 Codex 数据目录后开始扫描",
    emptyBody: "第一阶段只读取本地数据；导出、备份和报告只写入你选择的位置。",
    evidence: "数据来源",
    metadata: "元数据",
    diagnostics: "诊断",
    transcript: "转录",
    systemLogs: "系统日志",
    qaRecords: "问答记录",
    qaRecordsHint: "用户与助手对话",
    noDiagnostics: "没有诊断项",
    noDiagnosticsForSession: "这个会话没有发现可见性风险。",
    noTranscript: "没有从 JSONL 转录文件中恢复到消息。",
    workspace: "工作区",
    canonicalPath: "规范路径",
    rolloutFile: "转录文件",
    source: "来源",
    missing: "缺失",
    present: "存在",
    recentRank: "最近排序 {rank}",
    jsonl: "JSONL",
    sqlite: "SQLite",
    index: "索引",
    globalState: "全局状态",
    confidence: "置信度",
    action: "建议",
    unknown: "未知",
    repairCenter: "恢复到 Codex",
    repairCenterSubtitle: "已自动生成恢复计划。真实写入前仍需要备份、dry-run 和分项确认。",
    generateRepairPlan: "生成修复计划",
    runDryRun: "运行 dry-run",
    applyOfficialRepair: "执行官方修复",
    applyWorkspacePatch: "应用 workspace hint 补丁",
    applyJsonlMigration: "应用 JSONL migration",
    verifyRepair: "验证",
    rollbackRepair: "回滚",
    backupPathRequired: "需要先选择备份输出和 rollback 目录。",
    selectBackupAndRollback: "选择备份与回滚目录",
    closeCodexConfirm: "我已关闭 Codex Desktop",
    repairOperations: "修复操作",
    preconditions: "前置条件",
    dryRun: "Dry-run",
    verification: "验证结果",
    rollbackPackage: "回滚包",
    affectedFiles: "影响文件",
    risk: "风险",
    officialApi: "官方 API",
    localPatch: "本地补丁",
    experimental: "实验",
    noRepairPlan: "正在准备恢复计划...",
    noOperations: "没有可执行修复操作。",
    close: "关闭",
    operationCount: "{count} 个操作",
    threadCount: "{count} 个会话",
    fileCount: "{count} 个文件",
    backupRollback: "备份与回滚",
    dryRunNotStarted: "尚未运行 dry-run。dry-run 只会复制 CODEX_HOME 到临时目录，不会修改原始数据。",
    appServerListed: "app-server 列出 {active} 个未归档 / {archived} 个已归档会话",
    noAppServerResult: "没有 app-server 结果",
    readValidationCount: "{ok}/{count} 个会话读取成功",
    preconditionAppServer: "已检测到 Codex 官方 app-server",
    preconditionBackup: "执行真实修复前必须创建完整备份",
    preconditionDryRun: "执行真实修复前必须先在副本上 dry-run",
    preconditionCodexClosed: "真实修复前必须关闭 Codex Desktop",
    riskLow: "低风险",
    riskMedium: "中风险",
    riskHigh: "高风险",
    riskCritical: "关键风险",
    opOfficialScan: "官方 app-server 扫描并修复索引",
    opNameSet: "用官方 API 修复标题：{title}",
    opUnarchive: "用官方 API 取消归档：{title}",
    opForkCopy: "恢复为新副本：{title}",
    opWorkspaceHints: "补齐工作区提示缓存",
    opIndexRebuild: "实验性重建 session_index",
    opThreadSource: "补齐旧会话 thread_source：{title}",
    opSqlitePatch: "SQLite 最后手段补丁",
    opOfficialScanDesc: "让 Codex 官方 app-server 运行 thread/list scan-and-repair，优先由 Codex 自己补齐陈旧或缺失的 metadata。",
    opNameSetDesc: "通过官方 thread/name/set 更新可见标题，不直接改 session_index。",
    opUnarchiveDesc: "通过官方 thread/unarchive 把归档会话恢复到未归档历史。",
    opForkCopyDesc: "通过官方 thread/fork 创建一个新会话副本，不修改原始会话。",
    opWorkspaceHintsDesc: "只向 .codex-global-state.json 新增缺失的 thread-workspace-root-hints，不覆盖已有值，并跳过归档会话。",
    opIndexRebuildDesc: "高风险后备方案，MVP 中仅展示计划，不执行写入。",
    opThreadSourceDesc: "仅修改选中 JSONL 第一行 session_meta，为旧格式补齐 payload.thread_source。",
    opSqlitePatchDesc: "关键风险最后手段，MVP 未实现。",
    succeeded: "成功",
    failed: "失败"
  },
  en: {
    appName: "Codex Recall",
    readOnlyBadge: "Read-only mode",
    homePlaceholder: "Select Codex data directory",
    manualHome: "Manual",
    chooseHome: "Choose folder",
    actions: "Actions",
    clearSearch: "Clear search",
    collapseSidebar: "Collapse session list",
    expandSidebar: "Expand session list",
    shownInCodex: "Shown in Codex",
    archivedInCodex: "Archived in Codex",
    localOnly: "Local only",
    metadataOnly: "Metadata only",
    scan: "Scan",
    export: "Export",
    backup: "Backup",
    report: "Report",
    repair: "Repair Center",
    restoreToCodex: "Restore to Codex",
    restoreToCodexHint: "Generate a safe repair plan for this local-only session",
    language: "Language",
    chinese: "中文",
    english: "EN",
    scanStarted: "Scanning local Codex data",
    scanFinished: "Scan finished",
    scanStageStart: "Preparing scan",
    scanStageSnapshot: "Creating read-only snapshot",
    scanStageJsonl: "Reading transcript files",
    scanStageIndex: "Reading session index",
    scanStageSqlite: "Reading SQLite state",
    scanStageGlobalState: "Reading global state",
    scanStageNormalize: "Normalizing sessions",
    scanStageDiagnostics: "Applying diagnostics",
    scanStageSummary: "Preparing scan summary",
    scanStageProcessing: "Preparing results",
    scanComplete: "Scan complete: {count} sessions",
    exportDone: "Export written to {path}",
    backupDone: "Backup written to {path}",
    reportDone: "Recovery report written to {path}",
    repairPlanReady: "Repair plan ready: {count} operations",
    dryRunDone: "Dry-run finished: {status}",
    applyDone: "Repair apply finished: {status}",
    rollbackDone: "Rollback finished: {count} files",
    sessions: "Sessions",
    active: "Active",
    archived: "Archived",
    highRisk: "High risk",
    mediumRisk: "Medium risk",
    transcriptFiles: "Transcript files",
    databaseRows: "Database rows",
    workspaces: "Workspaces",
    searchPlaceholder: "Search title, body, path, thread ID, or diagnostic:R012 / risk:medium",
    noTimestamp: "No timestamp",
    unknownWorkspace: "Unknown workspace",
    selected: "Selected",
    all: "All sessions",
    exportPanel: "Export",
    exportSelected: "Export selected",
    exportAll: "Export all",
    redact: "Redact paths and likely secrets",
    backupReport: "Backup & report",
    createBackup: "Create zip backup",
    generateReport: "Generate recovery report",
    noSessionTitle: "No session selected",
    noSessionBody: "Select a session to inspect transcript, metadata, and diagnostics.",
    emptyTitle: "Select a Codex data directory and scan",
    emptyBody: "Phase 1 only reads local data. Exports, backups, and reports write only to locations you choose.",
    evidence: "Evidence",
    metadata: "Metadata",
    diagnostics: "Diagnostics",
    transcript: "Transcript",
    systemLogs: "System logs",
    qaRecords: "Q&A",
    qaRecordsHint: "User and assistant messages",
    noDiagnostics: "No diagnostics",
    noDiagnosticsForSession: "No visibility risk was found for this session.",
    noTranscript: "No messages were recovered from JSONL transcript files.",
    workspace: "Workspace",
    canonicalPath: "Canonical path",
    rolloutFile: "Transcript file",
    source: "Source",
    missing: "Missing",
    present: "Present",
    recentRank: "Recent rank {rank}",
    jsonl: "JSONL",
    sqlite: "SQLite",
    index: "Index",
    globalState: "Global state",
    confidence: "Confidence",
    action: "Action",
    unknown: "Unknown",
    repairCenter: "Restore to Codex",
    repairCenterSubtitle: "A repair plan is generated automatically. Real writes still require backup, dry-run, and selected apply.",
    generateRepairPlan: "Generate repair plan",
    runDryRun: "Run dry-run",
    applyOfficialRepair: "Apply official repair",
    applyWorkspacePatch: "Apply workspace hint patch",
    applyJsonlMigration: "Apply JSONL migration",
    verifyRepair: "Verify",
    rollbackRepair: "Rollback",
    backupPathRequired: "Choose backup output and rollback directory first.",
    selectBackupAndRollback: "Choose backup and rollback paths",
    closeCodexConfirm: "I have closed Codex Desktop",
    repairOperations: "Repair operations",
    preconditions: "Preconditions",
    dryRun: "Dry-run",
    verification: "Verification",
    rollbackPackage: "Rollback package",
    affectedFiles: "Affected files",
    risk: "Risk",
    officialApi: "Official API",
    localPatch: "Local patch",
    experimental: "Experimental",
    noRepairPlan: "Preparing repair plan...",
    noOperations: "No repair operations are available.",
    close: "Close",
    operationCount: "{count} operations",
    threadCount: "{count} thread",
    fileCount: "{count} files",
    backupRollback: "Backup and rollback",
    dryRunNotStarted: "Dry-run has not run yet. Dry-run copies CODEX_HOME to a temporary directory and does not modify original data.",
    appServerListed: "app-server listed {active} active / {archived} archived sessions",
    noAppServerResult: "No app-server result",
    readValidationCount: "{ok}/{count} reads succeeded",
    preconditionAppServer: "Codex official app-server detected",
    preconditionBackup: "A full backup is required before real repair",
    preconditionDryRun: "Dry-run on a copy is required before real repair",
    preconditionCodexClosed: "Codex Desktop must be closed before real repair",
    riskLow: "Low",
    riskMedium: "Medium",
    riskHigh: "High",
    riskCritical: "Critical",
    opOfficialScan: "Official app-server scan-and-repair",
    opNameSet: "Set official title: {title}",
    opUnarchive: "Unarchive with official API: {title}",
    opForkCopy: "Restore as a copy: {title}",
    opWorkspaceHints: "Patch workspace hint cache",
    opIndexRebuild: "Experimental session_index rebuild",
    opThreadSource: "Backfill thread_source: {title}",
    opSqlitePatch: "SQLite last-resort patch",
    opOfficialScanDesc: "Runs official thread/list scan-and-repair so Codex can repair stale or missing metadata itself.",
    opNameSetDesc: "Uses official thread/name/set to update the visible title without directly editing session_index.",
    opUnarchiveDesc: "Uses official thread/unarchive to restore an archived thread to active history.",
    opForkCopyDesc: "Uses official thread/fork to create a new visible copy without editing the original thread.",
    opWorkspaceHintsDesc: "Adds only missing thread-workspace-root-hints in .codex-global-state.json, never overwriting existing values and skipping archived sessions.",
    opIndexRebuildDesc: "High-risk fallback. In the MVP this is plan-only and does not write.",
    opThreadSourceDesc: "Updates only the first session_meta line of the selected JSONL to add payload.thread_source.",
    opSqlitePatchDesc: "Critical last-resort patch. Not implemented in the MVP.",
    succeeded: "Succeeded",
    failed: "Failed"
  }
} as const;

type DictionaryKey = keyof typeof dictionary.zh;

const riskLabels: Record<Language, Record<VisibilityRisk, string>> = {
  zh: {
    high: "高风险",
    medium: "中风险",
    low: "低风险"
  },
  en: {
    high: "High",
    medium: "Medium",
    low: "Low"
  }
};

const roleLabels: Record<Language, Record<MessageRole, string>> = {
  zh: {
    user: "用户",
    assistant: "助手",
    system: "系统",
    tool: "工具",
    unknown: "未知"
  },
  en: {
    user: "User",
    assistant: "Assistant",
    system: "System",
    tool: "Tool",
    unknown: "Unknown"
  }
};

const workspaceLabels: Record<Language, Record<string, string>> = {
  zh: {
    all: "全部会话",
    active: "未归档",
    archived: "已归档",
    "possibly-hidden": "可能不可见",
    "risk-high": "高风险",
    "risk-medium": "中风险",
    "unknown-workspace": "未识别工作区"
  },
  en: {
    all: "All sessions",
    active: "Active",
    archived: "Archived",
    "possibly-hidden": "Possibly hidden",
    "risk-high": "High risk",
    "risk-medium": "Medium risk",
    "unknown-workspace": "Unknown workspace"
  }
};

interface DiagnosticCopy {
  title: string;
  body: string;
  action: string;
}

const diagnosticCopies: Record<Language, Record<string, DiagnosticCopy>> = {
  zh: {
    R001_JSONL_EXISTS_SQLITE_MISSING: {
      title: "只有转录文件，数据库没有记录",
      body: "本地 JSONL 转录文件还在，但 SQLite 侧栏数据库里没有对应线程。Codex 桌面端可能因此不显示它。",
      action: "可以先在本工具查看、搜索和导出；不要手工编辑 SQLite。"
    },
    R002_SQLITE_EXISTS_JSONL_MISSING: {
      title: "数据库有记录，转录文件缺失",
      body: "SQLite 中有线程元数据，但没有找到对应 JSONL 转录文件。",
      action: "检查是否选错 Codex 目录、是否在归档目录或旧备份中。"
    },
    R003_INDEX_MISSING: {
      title: "索引文件缺少这条会话",
      body: "会话在转录文件或数据库中存在，但 session_index.jsonl 没有记录。",
      action: "先导出和备份；第一阶段不会重建索引。"
    },
    R004_DUPLICATE_INDEX: {
      title: "索引里有重复记录",
      body: "session_index.jsonl 中同一个线程 ID 出现了多次，可能导致标题或排序不一致。",
      action: "查看诊断报告确认重复行；第一阶段不修改索引。"
    },
    R005_ARCHIVED_FLAG_MISMATCH: {
      title: "归档状态不一致",
      body: "文件所在目录和 SQLite 记录的归档状态不一致。",
      action: "先导出和备份；不要手工移动原始会话文件。"
    },
    R006_ACTIVE_FILE_BUT_DB_ARCHIVED: {
      title: "文件在未归档目录，但数据库认为已归档",
      body: "JSONL 文件位于 sessions/，但 SQLite archived=true。",
      action: "生成报告并保留备份；第一阶段不修复归档状态。"
    },
    R007_ARCHIVED_FILE_BUT_DB_ACTIVE: {
      title: "文件在归档目录，但数据库认为未归档",
      body: "JSONL 文件位于 archived_sessions/，但 SQLite archived=false。",
      action: "生成报告并保留备份；第一阶段不修复归档状态。"
    },
    R008_CWD_EMPTY_OR_UNKNOWN: {
      title: "无法识别工作区",
      body: "没有从转录文件、数据库或全局状态中找到 cwd/workspace 信息。",
      action: "仍可查看和导出，但工作区分组可能不准确。"
    },
    R009_WINDOWS_EXTENDED_PATH_SPLIT: {
      title: "Windows 路径写法导致分组分裂",
      body: "同一工作区可能同时出现 \\\\?\\C:\\path 和 C:\\path 等写法。",
      action: "本工具会按规范路径合并显示；第一阶段不写回路径。"
    },
    R010_SYMLINK_OR_REALPATH_ALIAS: {
      title: "符号链接路径可能导致分组分裂",
      body: "原始 cwd 和真实路径不同，可能让桌面端把同一项目分成多个工作区。",
      action: "对比原始路径和规范路径；第一阶段不改写路径。"
    },
    R011_OUTSIDE_RECENT_50: {
      title: "不在最近 50 条会话内",
      body: "会话按更新时间排序已经排到 50 名之后，某些侧栏问题可能只加载最近窗口。",
      action: "可在本工具中继续查看和导出旧会话。"
    },
    R012_WORKSPACE_HINT_MISSING: {
      title: "缺少工作区提示",
      body: "会话有 cwd 信息，但全局状态里没有 thread-workspace-root-hints 记录。Codex 项目分组可能因此漏掉它。",
      action: "生成恢复报告；第一阶段不会修改全局状态。"
    },
    R013_PROJECTLESS_THREAD_ID_MISSING: {
      title: "可能缺少 projectless 列表记录",
      body: "全局状态存在 projectless-thread-ids，但这条未归档会话不在列表中。",
      action: "这只是可疑信号；第一阶段不修补该列表。"
    },
    R014_TITLE_DRIFT: {
      title: "标题来源不一致",
      body: "索引、SQLite 和转录首条消息推断出的标题不一致。",
      action: "本工具会选择较可信标题展示；第一阶段不回写标题。"
    },
    R015_MALFORMED_JSONL: {
      title: "转录文件存在坏行",
      body: "JSONL 文件里有无法解析的行，但工具会尽量保留可读内容。",
      action: "尽快备份原始文件，并导出能恢复的消息。"
    },
    R016_SUSPICIOUS_BOGUS_STATUS_SESSION: {
      title: "疑似状态噪声会话",
      body: "会话像是 status/root 之类的噪声记录，可能挤占最近会话列表。",
      action: "第一阶段不要删除；先生成报告。"
    },
    R017_SQLITE_INTEGRITY_FAILED: {
      title: "SQLite 数据库损坏或无法读取",
      body: "state_*.sqlite 完整性检查失败，或无法以只读方式打开。",
      action: "先完整备份 Codex 数据，再用 JSONL 转录文件查看和导出。"
    },
    R018_ORPHAN_INDEX_RECORD: {
      title: "孤立索引记录",
      body: "session_index.jsonl 有线程 ID，但没有找到对应转录文件或 SQLite 记录。",
      action: "可能是过期元数据；第一阶段不删除或修补。"
    },
    R019_APP_SERVER_LIST_MISMATCH: {
      title: "app-server 列表未显示",
      body: "本工具能看到会话，但官方 app-server thread/list 没有列出它。",
      action: "先运行官方 scan-and-repair dry-run，再检查索引和缓存补丁。"
    },
    R020_APP_SERVER_READ_FAILED: {
      title: "app-server 读取失败",
      body: "本地转录证据存在，但官方 thread/read 失败。",
      action: "不要盲修；优先检查 JSONL metadata、坏行和归档状态。"
    },
    R021_APP_SERVER_RESUME_FAILED: {
      title: "app-server resume 失败",
      body: "thread/read 可读，但 resume 失败。",
      action: "优先考虑 thread/fork 恢复为副本，而不是改旧记录。"
    },
    R022_MISSING_THREAD_SOURCE: {
      title: "缺少 thread_source",
      body: "旧 JSONL 的 session_meta payload 缺少 thread_source。",
      action: "只在备份、dry-run 和验证后做第一行 metadata migration。"
    },
    R023_WORKSPACE_HINT_RECOVERABLE: {
      title: "工作区提示可恢复",
      body: "缺少 workspace hint，但可以从 cwd 或 writable roots 推断。",
      action: "生成只新增、不覆盖、跳过归档会话的 JSON patch。"
    },
    R024_RECENT_WINDOW_HIDDEN: {
      title: "可能被最近窗口隐藏",
      body: "会话有效，但可能不在桌面端侧栏最近加载窗口内。",
      action: "不要批量改时间；对选中会话可考虑 fork 副本。"
    },
    R025_INDEX_STALE_BUT_APP_SERVER_CAN_READ: {
      title: "索引陈旧但 app-server 可读",
      body: "session_index 可能陈旧，但官方 thread/read 可以读取会话。",
      action: "优先使用 app-server repair 和 thread/name/set。"
    },
    R026_SQLITE_STALE_BUT_ROLLOUT_VALID: {
      title: "SQLite 陈旧但 rollout 有效",
      body: "SQLite metadata 缺失或陈旧，但 JSONL rollout 有效。",
      action: "优先运行官方 scan-and-repair，不直接写 SQLite。"
    }
  },
  en: {
    R001_JSONL_EXISTS_SQLITE_MISSING: {
      title: "Transcript exists, database row missing",
      body: "A JSONL transcript exists, but no matching SQLite thread row was found. Codex Desktop may not show it in the sidebar.",
      action: "View and export it here first. Do not manually edit SQLite."
    },
    R002_SQLITE_EXISTS_JSONL_MISSING: {
      title: "Database row exists, transcript missing",
      body: "SQLite contains metadata, but no matching JSONL transcript was found.",
      action: "Check archived sessions, backups, and whether the selected Codex directory is correct."
    },
    R003_INDEX_MISSING: {
      title: "Missing from session index",
      body: "The session exists in JSONL or SQLite, but session_index.jsonl has no matching record.",
      action: "Export and back up first. Phase 1 does not rebuild the index."
    },
    R004_DUPLICATE_INDEX: {
      title: "Duplicate index records",
      body: "The same thread ID appears multiple times in session_index.jsonl, which may affect stale titles or ordering.",
      action: "Review the report. Phase 1 does not modify the index."
    },
    R005_ARCHIVED_FLAG_MISMATCH: {
      title: "Archive state mismatch",
      body: "Path evidence and SQLite metadata disagree on whether the session is archived.",
      action: "Export and back up first. Do not move original session files manually."
    },
    R006_ACTIVE_FILE_BUT_DB_ARCHIVED: {
      title: "Active file, archived in database",
      body: "The JSONL file is under sessions/, but SQLite says archived=true.",
      action: "Generate a report and keep a backup. Phase 1 does not repair archive state."
    },
    R007_ARCHIVED_FILE_BUT_DB_ACTIVE: {
      title: "Archived file, active in database",
      body: "The JSONL file is under archived_sessions/, but SQLite says archived=false.",
      action: "Generate a report and keep a backup. Phase 1 does not repair archive state."
    },
    R008_CWD_EMPTY_OR_UNKNOWN: {
      title: "Workspace unknown",
      body: "No cwd/workspace information was found in JSONL, SQLite, or global state.",
      action: "The session can still be viewed/exported, but workspace grouping may be unreliable."
    },
    R009_WINDOWS_EXTENDED_PATH_SPLIT: {
      title: "Windows path spellings may split a workspace",
      body: "The same workspace may appear as both \\\\?\\C:\\path and C:\\path variants.",
      action: "This tool groups by canonical path. Phase 1 does not write paths back."
    },
    R010_SYMLINK_OR_REALPATH_ALIAS: {
      title: "Symlink or realpath alias risk",
      body: "Raw cwd and realpath differ, which may split one project into multiple workspaces.",
      action: "Compare raw and canonical paths. Phase 1 does not rewrite paths."
    },
    R011_OUTSIDE_RECENT_50: {
      title: "Outside recent 50 sessions",
      body: "The session is older than the top 50 active sessions by update time.",
      action: "Use this tool to view and export older sessions."
    },
    R012_WORKSPACE_HINT_MISSING: {
      title: "Workspace hint missing",
      body: "The session has cwd evidence, but global state has no thread-workspace-root-hints entry. Codex project grouping may omit it.",
      action: "Generate a recovery report. Phase 1 does not patch global state."
    },
    R013_PROJECTLESS_THREAD_ID_MISSING: {
      title: "Projectless list may be missing the thread",
      body: "projectless-thread-ids exists, but this active local thread is not listed.",
      action: "Treat this as a suspicious signal only. Phase 1 does not patch the list."
    },
    R014_TITLE_DRIFT: {
      title: "Title drift",
      body: "Index, SQLite, and transcript-derived title candidates differ.",
      action: "This tool displays the chosen title. Phase 1 does not write titles back."
    },
    R015_MALFORMED_JSONL: {
      title: "Malformed transcript lines",
      body: "The JSONL transcript contains malformed lines; readable events are still recovered when possible.",
      action: "Back up original files and export recovered messages."
    },
    R016_SUSPICIOUS_BOGUS_STATUS_SESSION: {
      title: "Suspicious status/noise session",
      body: "The session resembles a status/root noise record that may crowd out recent sessions.",
      action: "Do not delete in Phase 1. Generate a report first."
    },
    R017_SQLITE_INTEGRITY_FAILED: {
      title: "SQLite integrity failed",
      body: "A state_*.sqlite database failed integrity check or could not be opened read-only.",
      action: "Back up Codex data and use JSONL transcript files for viewing/export."
    },
    R018_ORPHAN_INDEX_RECORD: {
      title: "Orphan index record",
      body: "session_index.jsonl contains a thread ID, but no JSONL or SQLite evidence was found.",
      action: "This may be stale metadata. Phase 1 does not delete or patch it."
    },
    R019_APP_SERVER_LIST_MISMATCH: {
      title: "App-server list mismatch",
      body: "The local scanner can see the session, but official thread/list did not list it.",
      action: "Run official scan-and-repair dry-run before considering index or cache patches."
    },
    R020_APP_SERVER_READ_FAILED: {
      title: "App-server read failed",
      body: "Local transcript evidence exists, but official thread/read failed.",
      action: "Do not patch blindly. Check JSONL metadata, malformed lines, and archive state."
    },
    R021_APP_SERVER_RESUME_FAILED: {
      title: "App-server resume failed",
      body: "thread/read can read the session, but resume failed.",
      action: "Prefer thread/fork restore-as-copy over editing the original record."
    },
    R022_MISSING_THREAD_SOURCE: {
      title: "Missing thread_source",
      body: "The legacy JSONL session_meta payload is missing thread_source.",
      action: "Only run first-line metadata migration after backup, dry-run, and validation."
    },
    R023_WORKSPACE_HINT_RECOVERABLE: {
      title: "Workspace hint recoverable",
      body: "The workspace hint is missing but can be inferred from cwd or writable roots.",
      action: "Generate an add-only JSON patch that skips archived sessions."
    },
    R024_RECENT_WINDOW_HIDDEN: {
      title: "Likely hidden by recent window",
      body: "The session is valid but may be outside the Desktop sidebar preload window.",
      action: "Do not bulk edit timestamps. Consider fork-as-copy for selected sessions."
    },
    R025_INDEX_STALE_BUT_APP_SERVER_CAN_READ: {
      title: "Index stale, app-server can read",
      body: "session_index may be stale, but official thread/read can read the session.",
      action: "Prefer app-server repair and thread/name/set."
    },
    R026_SQLITE_STALE_BUT_ROLLOUT_VALID: {
      title: "SQLite stale, rollout valid",
      body: "SQLite metadata is missing or stale, but the JSONL rollout is valid.",
      action: "Run official scan-and-repair instead of direct SQLite writes."
    }
  }
};

export function useI18n() {
  const [language, setLanguageState] = useState<Language>(() => {
    const saved = window.localStorage.getItem(storageKey);
    if (saved === "zh" || saved === "en") return saved;
    return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
  });

  useEffect(() => {
    window.localStorage.setItem(storageKey, language);
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  }, [language]);

  return useMemo(() => {
    const t = (key: DictionaryKey, params?: Record<string, string | number>) => {
      let value: string = dictionary[language][key] || dictionary.en[key];
      if (params) {
        for (const [name, replacement] of Object.entries(params)) {
          value = value.replace(`{${name}}`, String(replacement));
        }
      }
      return value;
    };
    return {
      language,
      setLanguage: setLanguageState,
      t,
      riskLabel: (risk: VisibilityRisk) => riskLabels[language][risk],
      roleLabel: (role: MessageRole) => roleLabels[language][role],
      workspaceLabel: (group: Pick<WorkspaceGroup, "key" | "display_path">) =>
        workspaceLabels[language][group.key] || group.display_path,
      diagnosticCopy: (diagnostic: Diagnostic): DiagnosticCopy => {
        const copy = diagnosticCopies[language][diagnostic.code];
        if (copy) return copy;
        return {
          title: diagnostic.code,
          body: diagnostic.message,
          action: diagnostic.suggested_action || ""
        };
      }
    };
  }, [language]);
}

export type I18n = ReturnType<typeof useI18n>;
