import { describe, expect, it } from "vitest";
import { parseChecksums, validateSpdx } from "./verify-release.mjs";

describe("release verification", () => {
  it("accepts strict checksum lines and rejects paths", () => {
    const digest = "a".repeat(64);
    expect(parseChecksums(`${digest}  artifact.exe\n`).get("artifact.exe")).toBe(digest);
    expect(() => parseChecksums(`${digest}  ../artifact.exe\n`)).toThrow("Invalid SHA256SUMS");
  });

  it("rejects duplicate checksum entries", () => {
    const digest = "b".repeat(64);
    expect(() => parseChecksums(`${digest}  artifact.exe\n${digest}  artifact.exe\n`)).toThrow("Duplicate checksum");
  });

  it("validates SPDX package, license and relationship coverage", () => {
    const spdx = {
      spdxVersion: "SPDX-2.3", dataLicense: "CC0-1.0",
      packages: [{ SPDXID: "SPDXRef-App", licenseDeclared: "AGPL-3.0-only" }, { SPDXID: "SPDXRef-Dep", licenseDeclared: "MIT" }],
      relationships: [{}, {}],
    };
    expect(() => validateSpdx(spdx, { dependencies: { total: 1 } })).not.toThrow();
    expect(() => validateSpdx({ ...spdx, packages: [...spdx.packages, { SPDXID: "SPDXRef-Dep", licenseDeclared: "MIT" }] }, { dependencies: { total: 2 } })).toThrow("not unique");
  });
});
