import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatDuration,
  grouped,
  progressLabel,
  shortDefinitionLabel,
  validateVariableValue,
} from "./format";

describe("format", () => {
  it("formats bytes like the desktop app", () => {
    expect(formatBytes(0)).toBe("0 Б");
    expect(formatBytes(512)).toBe("512 Б");
    expect(formatBytes(1536)).toBe("1,5 КБ");
    expect(formatBytes(3 * 1024 * 1024)).toBe("3 МБ");
  });

  it("formats durations", () => {
    expect(formatDuration(null)).toBe("Нет данных");
    expect(formatDuration(7 * 60_000)).toBe("7 мин.");
    expect(formatDuration(3 * 3_600_000 + 12 * 60_000)).toBe("3 ч. 12 мин.");
    expect(formatDuration(2 * 86_400_000 + 4 * 3_600_000)).toBe("2 д. 4 ч.");
  });

  it("groups thousands and shortens definition ids", () => {
    expect(grouped(1234567)).toBe("1 234 567");
    expect(shortDefinitionLabel("dover:12:abc")).toBe("dover · v12");
    expect(shortDefinitionLabel("plain")).toBe("plain");
  });

  it("validates variable values by type", () => {
    const variable = { name: "x", type: "Integer", value: "1", valueInfo: null };
    expect(validateVariableValue(variable, "42")).toBeNull();
    expect(validateVariableValue(variable, "4.2")).not.toBeNull();
    expect(validateVariableValue({ ...variable, type: "Boolean" }, "TRUE")).toBeNull();
    expect(validateVariableValue({ ...variable, type: "Long" }, "9223372036854775808")).not.toBeNull();
  });

  it("describes download progress", () => {
    expect(
      progressLabel({ stage: "Downloading", downloadedBytes: 512, totalBytes: 1024, bytesPerSecond: 256, destinationPath: "" }),
    ).toBe("50% · осталось 512 Б · ≈ 2 с");
    expect(
      progressLabel({ stage: "Verifying", downloadedBytes: 1, totalBytes: 1, bytesPerSecond: 0, destinationPath: "" }),
    ).toBe("Проверяем SHA-256…");
  });
});
