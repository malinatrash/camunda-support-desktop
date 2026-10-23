import { describe, expect, it } from "vitest";
import { groupIncidents, incidentSignature, relativeTime } from "./incidents";
import type { ProcessIncident } from "./types";

function incident(overrides: Partial<ProcessIncident>): ProcessIncident {
  return {
    id: "i",
    type: "failedExternalTask",
    activityId: "Task_a",
    failedActivityId: null,
    message: null,
    timestamp: null,
    configuration: "cfg",
    processDefinitionId: "dover:3:abc",
    processInstanceId: "p1",
    executionId: null,
    tenantId: null,
    annotation: null,
    ...overrides,
  };
}

describe("incidents", () => {
  it("normalizes messages into signatures", () => {
    expect(incidentSignature("Timeout for 9d1f6e2a-0001-4c3e-9a1f-aabbccddeeff after 30000 ms\nstack")).toBe("Timeout for # after # ms");
    expect(incidentSignature(null)).toBe("Без сообщения");
  });

  it("groups incidents by activity, process and signature", () => {
    const groups = groupIncidents([
      incident({ id: "1", message: "Connection refused host-1", timestamp: "2026-10-23T10:00:00.000+0500" }),
      incident({ id: "2", message: "Connection refused host-2", processInstanceId: "p2", timestamp: "2026-10-23T11:00:00.000+0500" }),
      incident({ id: "3", message: "Other", activityId: "Task_b" }),
    ]);
    expect(groups).toHaveLength(2);
    expect(groups[0].count).toBe(2);
    expect(groups[0].instanceIds).toEqual(["p1", "p2"]);
    expect(groups[0].latestTimestamp).toBe("2026-10-23T11:00:00.000+0500");
    expect(groups[0].definitionKey).toBe("dover");
  });

  it("formats relative time", () => {
    const now = Date.parse("2026-10-23T12:00:00.000+05:00");
    expect(relativeTime("2026-10-23T11:57:00.000+0500", now)).toBe("3 мин назад");
    expect(relativeTime("2026-10-22T09:00:00.000+0500", now)).toBe("27 ч назад");
    expect(relativeTime(null, now)).toBe("—");
  });
});
