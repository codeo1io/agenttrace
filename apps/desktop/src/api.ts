import { invoke } from "@tauri-apps/api/core";
import type { CompareData, Finding, HomeData, SessionDetail, SessionSummary, SourceState } from "./types";
import { mockApi } from "./mock";

const tauriApi = {
  refreshSessions: () => invoke<void>("refresh_sessions"),
  detectSources: () => invoke<SourceState>("detect_sources"),
  loadHome: () => invoke<HomeData>("load_home"),
  listSessions: (query = "", statusFilter = "all") =>
    invoke<{ sessions: SessionSummary[]; total: number }>("list_sessions", { query, statusFilter }),
  getSession: (id: string) => invoke<SessionDetail>("get_session", { id }),
  listFindings: () => invoke<Finding[]>("list_findings"),
  compareSessions: (id: string, previousId?: string) => invoke<CompareData>("compare_sessions", { id, previousId }),
};

export const api = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window ? tauriApi : mockApi;
