import { describe, expect, it } from "vitest";
import { sessionsInRange } from "./App";
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
});
