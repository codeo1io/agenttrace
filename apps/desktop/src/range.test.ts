import { describe, expect, it } from "vitest";
import { compactTokens, healthTrend, sessionsInRange } from "./App";
import type { SessionSummary } from "./types";

const session = (id: string, startedAt: string) => ({ id, startedAt } as SessionSummary);

describe("home time range", () => {
  it("filters loaded sessions without reloading data", () => {
    const now = Date.parse("2026-07-19T12:00:00Z");
    const sessions = [
      session("recent", "2026-07-19T11:00:00Z"),
      session("week", "2026-07-16T12:00:00Z"),
      session("old", "2026-06-01T12:00:00Z"),
      session("invalid", "unknown"),
    ];

    expect(sessionsInRange(sessions, 24, now).map(({ id }) => id)).toEqual(["recent"]);
    expect(sessionsInRange(sessions, 168, now).map(({ id }) => id)).toEqual(["recent", "week"]);
  });

  it("builds the trend from session health", () => {
    const sessions = [
      { ...session("a", "2026-07-19T10:00:00Z"), health: 50 },
      { ...session("b", "2026-07-19T11:00:00Z"), health: 100 },
    ];
    expect(healthTrend(sessions)).toBe("M0 120 L800 0");
  });

  it("formats tokens with K, M, and T units", () => {
    expect([compactTokens(1_200), compactTokens(2_500_000), compactTokens(3_000_000_000_000)]).toEqual(["1.2K", "2.5M", "3.0T"]);
  });
});
