import { describe, expect, it } from "vitest";
import { DEFAULT_LANGUAGE, evidenceLabel, findingCopy, outcomeLabel, reasonLabel, useCopy } from "./i18n";
import type { Finding } from "./types";

describe("desktop i18n", () => {
  it("defaults to Simplified Chinese", () => { expect(DEFAULT_LANGUAGE).toBe("zh"); expect(useCopy(DEFAULT_LANGUAGE).home).toBe("首页"); });
  it("keeps primary navigation aligned", () => {
    const zh=useCopy("zh"), en=useCopy("en");
    expect([zh.home,zh.sessions,zh.discover,zh.compare]).toEqual(["首页","会话","发现","对比"]);
    expect([en.home,en.sessions,en.discover,en.compare]).toEqual(["Home","Sessions","Discover","Compare"]);
  });
  it("translates every backend code in both languages", () => {
    const findingKinds=["loop","retry","latency","context","large_params","stuck","cost"];
    for(const kind of findingKinds){ const finding={kind,value:2,detail:"tool"} as Finding; expect(findingCopy(finding,"zh").title).not.toBe(kind); expect(findingCopy(finding,"en").title).not.toBe(kind); }
    for(const code of ["faster_cheaper","faster_costlier","slower_cheaper","slower_costlier"]){ expect(outcomeLabel(code,"zh")).not.toBe(code); expect(outcomeLabel(code,"en")).not.toBe(code); }
    for(const code of ["fewer_failures","less_repeated_work","review_metric_changes"]){ expect(reasonLabel(code,"zh")).not.toBe(code); expect(reasonLabel(code,"en")).not.toBe(code); }
    for(const code of ["repeated_groups","failed_calls","slowest_tool","context_used","largest_input","pattern","session_cost"]){ expect(evidenceLabel(code,"zh")).not.toBe(code); expect(evidenceLabel(code,"en")).not.toBe(code); }
  });
  it("states the local-only privacy boundary", () => { expect(useCopy("zh").privacy).toContain("本地"); expect(useCopy("en").privacy).toContain("device"); });
});
