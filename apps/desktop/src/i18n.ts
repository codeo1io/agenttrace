import type { Finding } from "./types";

export type Language = "zh" | "en";
export const DEFAULT_LANGUAGE: Language = "zh";

const copy = {
  zh: {
    home: "首页", sessions: "会话", discover: "发现", compare: "对比", settings: "设置", systemHealthy: "系统运行正常", dataNeedsAttention: "数据需要关注",
    welcome: "欢迎使用 AgentTrace", welcomeBody: "专为 AI 编程助手打造的可观测性工具", start: "开始使用", importData: "导入已有数据",
    privacy: "所有数据仅保存在本地，绝不上传。", tracking: "追踪每一次对话", trackingBody: "自动记录与 AI 助手的每一次交互，全面了解过程。",
    insights: "洞察性能表现", insightsBody: "关键指标一目了然，快速发现响应慢、错误多的环节。", opportunities: "发现改进机会",
    opportunitiesBody: "智能分析异常与瓶颈，获得可执行的优化建议。", optimize: "对比与优化", optimizeBody: "对比不同模型与版本，找到最适合你的搭配。",
    totalSessions: "会话总数", healthScore: "健康评分", issueRate: "需关注率", p95Latency: "P95 时长", spend: "成本", recent: "最近会话",
    healthTrend: "健康趋势", attention: "异常", viewAll: "查看全部", healthySessions: "健康会话", needsAttention: "需关注", all: "全部", smooth: "顺利",
    search: "搜索会话、项目或助手", refresh: "刷新", sessionCount: (n: number) => `共 ${n} 条会话`, noSessions: "没有匹配的会话。",
    found: (n: number) => `我们发现 ${n} 个可以改进的地方`, foundBody: "这些优化可以帮你节省时间、提升体验和降低成本。",
    evidence: "查看证据", improve: "如何改进", doingWell: "做得好的地方", doingWellBody: "错误率低，响应流畅，费用控制合理。",
    compareTitle: "对比两次会话", current: "这一次", previous: "上一次", versus: "对比", why: "为什么会更好", technical: "查看技术细节",
    noComparison: "至少需要两条会话才能进行对比。", appearance: "外观", language: "语言", rescan: "重新扫描", localSources: "本地数据源",
    light: "浅色", dark: "深色", detected: "已检测", notFound: "未找到", followPreference: "使用你的显示偏好", localOnlyBody: "AgentTrace 读取本地会话文件，无需账号。",
    scanning: "正在扫描本地会话…", loadFailed: "AgentTrace 无法加载会话", retry: "重试", duration: "完成时间", cost: "花费", toolFailures: "工具失败", repeatedWork: "重复操作",
    tokens: "令牌", health: "健康评分", saved: "节省", reduced: "减少", less: "少", more: "多", everythingGood: "一切正常", noRepeated: "没有需要关注的重复工作。",
  },
  en: {
    home: "Home", sessions: "Sessions", discover: "Discover", compare: "Compare", settings: "Settings", systemHealthy: "System running normally", dataNeedsAttention: "Data needs attention",
    welcome: "Welcome to AgentTrace", welcomeBody: "Observability built for AI coding assistants", start: "Get started", importData: "Import existing data",
    privacy: "All data stays on this device and is never uploaded.", tracking: "Trace every conversation", trackingBody: "Automatically record every interaction with your AI assistants.",
    insights: "Understand performance", insightsBody: "See key metrics and quickly spot slow or error-prone steps.", opportunities: "Find improvements",
    opportunitiesBody: "Analyze bottlenecks and get practical recommendations.", optimize: "Compare and optimize", optimizeBody: "Compare models and versions to find your best setup.",
    totalSessions: "Total sessions", healthScore: "Health score", issueRate: "Attention rate", p95Latency: "P95 duration", spend: "Cost", recent: "Recent sessions",
    healthTrend: "Health trend", attention: "Anomalies", viewAll: "View all", healthySessions: "Healthy sessions", needsAttention: "Needs attention", all: "All", smooth: "Smooth",
    search: "Search sessions, projects, or assistants", refresh: "Refresh", sessionCount: (n: number) => `${n} sessions`, noSessions: "No sessions match this search.",
    found: (n: number) => `We found ${n} ways to improve`, foundBody: "These changes can save time, improve the experience, and reduce cost.",
    evidence: "View evidence", improve: "How to improve", doingWell: "Doing well", doingWellBody: "Low error rate, responsive sessions, and stable cost.",
    compareTitle: "Compare two sessions", current: "This time", previous: "Previous", versus: "versus", why: "Why this is better", technical: "View technical details",
    noComparison: "At least two sessions are required for comparison.", appearance: "Appearance", language: "Language", rescan: "Scan again", localSources: "Local sources",
    light: "Light", dark: "Dark", detected: "Detected", notFound: "Not found", followPreference: "Use your display preference", localOnlyBody: "AgentTrace reads local session files and requires no account.",
    scanning: "Scanning local sessions…", loadFailed: "AgentTrace could not load sessions", retry: "Try again", duration: "Duration", cost: "Cost", toolFailures: "Tool failures", repeatedWork: "Repeated work",
    tokens: "Tokens", health: "Health", saved: "Saved", reduced: "Reduced", less: "Less", more: "More", everythingGood: "Everything looks good", noRepeated: "No repeated work needs attention.",
  },
} as const;

export type Copy = (typeof copy)[Language];
export function useCopy(language: Language): Copy { return copy[language]; }
export function locale(language: Language) { return language === "zh" ? "zh-CN" : "en-US"; }

const findingText = {
  zh: {
    loop: ["重复调用了同一个工具", (f: Finding) => `重复工作可能增加了 ${money(f.value, "zh")}。`, "连续得到相同结果后停止重试，换一种处理方式。"],
    retry: ["多次重试失败的工具", (f: Finding) => `${Math.round(f.value)} 次工具调用失败。`, "再次调用前先检查失败参数。"],
    latency: ["等待工具返回的时间较长", (f: Finding) => `${f.detail || "工具"} 最长耗时 ${duration(f.value, "zh")}。`, "为长时间任务设置超时，并保持可取消。"],
    context: ["对话内容接近上限", (f: Finding) => `预计已使用约 ${Math.round(f.value)}% 的上下文。`, "在达到上限前开启一个新会话。"],
    large_params: ["工具输入内容过大", (f: Finding) => `${f.detail || "工具"} 发送了 ${Math.round(f.value)} KB。`, "只传递相关字段，或拆分为更小的输入。"],
    stuck: ["助手可能陷入停滞", (f: Finding) => f.detail || "检测到重复模式。", "暂停运行，检查最后一个成功步骤后再继续。"],
    cost: ["这次会话的花费高于平时", (f: Finding) => f.detail || `本次花费 ${money(f.value, "zh")}。`, "检查高成本步骤并减少重复工作。"],
  },
  en: {
    loop: ["Repeated the same tool call", (f: Finding) => `Repeated work may have added ${money(f.value, "en")}.`, "Stop after identical results and change the approach."],
    retry: ["Retried failed tool calls", (f: Finding) => `${Math.round(f.value)} tool calls failed.`, "Inspect failed arguments before trying again."],
    latency: ["Waited longer for a tool", (f: Finding) => `${f.detail || "A tool"} took up to ${duration(f.value, "en")}.`, "Use a timeout and keep long-running work cancellable."],
    context: ["Conversation is getting full", (f: Finding) => `About ${Math.round(f.value)}% of the estimated context is in use.`, "Start a fresh session before reaching the limit."],
    large_params: ["Sent unusually large tool input", (f: Finding) => `${f.detail || "A tool"} sent ${Math.round(f.value)} KB.`, "Send only relevant fields or split the input."],
    stuck: ["The assistant may be stuck", (f: Finding) => f.detail || "A repeated pattern was detected.", "Pause, review the last successful step, and continue from there."],
    cost: ["This session cost more than usual", (f: Finding) => f.detail || `This session cost ${money(f.value, "en")}.`, "Review expensive steps and remove repeated work."],
  },
} as const;

export function findingCopy(finding: Finding, language: Language) {
  const fallback = language === "zh" ? ["这次会话需要关注", () => "请查看相关证据。", "打开证据并检查受影响的步骤。"] : ["This session needs attention", () => "Review the evidence.", "Open the evidence and review the affected step."];
  const text = (findingText[language] as Record<string, readonly [string, (f: Finding) => string, string]>)[finding.kind] || fallback;
  return { title: text[0], impact: text[1](finding), recommendation: text[2] };
}
export function evidenceLabel(kind: string, language: Language) { const labels: Record<string, [string,string]> = { repeated_groups:["重复组数","Repeated groups"], failed_calls:["失败调用","Failed calls"], slowest_tool:["最慢工具","Slowest tool"], context_used:["上下文占用","Context used"], largest_input:["最大输入","Largest input"], pattern:["模式","Pattern"], session_cost:["会话花费","Session cost"] }; return labels[kind]?.[language === "zh" ? 0 : 1] || kind; }
export function outcomeLabel(code: string, language: Language) { const labels: Record<string,[string,string]> = { faster_cheaper:["这一次更快，也更省。","This time was faster and less expensive."], faster_costlier:["这一次更快，但花费更多。","This time was faster, but cost more."], slower_cheaper:["这一次更省，但耗时更长。","This time cost less, but took longer."], slower_costlier:["这一次耗时更长，花费也更多。","This time took longer and cost more."] }; return labels[code]?.[language === "zh" ? 0 : 1] || code; }
export function reasonLabel(code: string, language: Language) { const labels: Record<string,[string,string]> = { fewer_failures:["工具调用失败更少。","Fewer tool calls failed."], less_repeated_work:["减少了重复操作和回滚。","Less work was repeated."], review_metric_changes:["查看技术细节以了解指标变化。","Open technical details to review metric changes."] }; return labels[code]?.[language === "zh" ? 0 : 1] || code; }
export function money(value: number, language: Language) { return new Intl.NumberFormat(locale(language), { style:"currency", currency:"USD", maximumFractionDigits:2 }).format(value); }
export function duration(seconds: number, language: Language) { if (seconds < 60) return language === "zh" ? `${Math.round(seconds)} 秒` : `${Math.round(seconds)} sec`; const m=Math.floor(seconds/60), s=Math.round(seconds%60); return language === "zh" ? `${m} 分 ${s} 秒` : `${m} min ${s} sec`; }
export function dateTime(value: string, language: Language) { const parsed=new Date(value); return Number.isNaN(parsed.valueOf()) ? "—" : new Intl.DateTimeFormat(locale(language), { month:"short", day:"numeric", hour:"2-digit", minute:"2-digit" }).format(parsed); }
