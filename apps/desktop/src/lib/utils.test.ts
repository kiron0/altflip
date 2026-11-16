import { describe, expect, it } from "vitest";
import { countdown, planLabel, relativeTime, rowState } from "./utils";
import type { AccountView } from "./api";

const base = { active: false, activationState: null, health: "ok", lastError: null } as unknown as AccountView;

describe("utils", () => {
  it("formats countdowns", () => {
    const now = Date.parse("2026-01-01T00:00:00Z");
    expect(countdown("2026-01-01T03:15:00Z", now)).toBe("3h 15m");
    expect(countdown("2026-01-03T01:00:00Z", now)).toBe("2d 1h");
    expect(countdown("2025-12-31T00:00:00Z", now)).toBe("now");
    expect(countdown(null, now)).toBeNull();
  });

  it("formats relative time", () => {
    const now = Date.parse("2026-01-01T01:00:00Z");
    expect(relativeTime("2026-01-01T00:00:00Z", now)).toBe("1 hour ago");
    expect(relativeTime(null, now)).toBe("never");
  });

  it("labels plans", () => {
    expect(planLabel("max")).toBe("Max");
    expect(planLabel("some_tier")).toBe("Some Tier");
    expect(planLabel(null)).toBeNull();
  });

  it("derives row state", () => {
    expect(rowState({ ...base, active: true, activationState: "verified" })).toBe("active");
    expect(rowState({ ...base, active: true, activationState: "drifted" })).toBe("drifted");
    expect(rowState({ ...base, health: "expired" })).toBe("expired");
    expect(rowState({ ...base, lastError: "x" })).toBe("error");
    expect(rowState(base)).toBe("idle");
  });
});
