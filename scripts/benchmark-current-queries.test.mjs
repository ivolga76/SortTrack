import { describe, expect, it } from "vitest";
import { parseQueryBenchmarkArguments, summarizeDurations } from "./benchmark-current-queries.mjs";

describe("current query benchmark", () => {
  it("summarizes sorted percentile boundaries", () => {
    expect(summarizeDurations([5, 1, 4, 2, 3])).toEqual({ minimumMs: 1, p50Ms: 3, p95Ms: 5, maximumMs: 5 });
  });

  it("requires an explicit database and bounded iterations", () => {
    expect(() => parseQueryBenchmarkArguments([])).toThrow("--database is required");
    expect(() => parseQueryBenchmarkArguments(["--database", "profile.db", "--iterations", "4"])).toThrow("between 5 and 500");
    expect(parseQueryBenchmarkArguments(["--database", "profile.db", "--iterations", "10"])).toMatchObject({ iterations: 10 });
  });
});
