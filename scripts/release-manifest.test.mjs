import { describe, expect, it } from "vitest";
import { buildSpdx, collectComponents, licenseCsv, sha256Buffer } from "./release-manifest.mjs";

describe("release manifest", () => {
  const packageLock = {
    name: "sorttrack",
    packages: {
      "": { name: "sorttrack", version: "0.1.0" },
      "node_modules/react": { version: "19.0.0", license: "MIT" },
      "node_modules/@scope/tool": { version: "1.2.3", license: "Apache-2.0" },
    },
  };
  const cargoPackages = [
    { name: "sorttrack", version: "0.1.0", license: "AGPL-3.0-only", source: null },
    { name: "rusqlite", version: "0.38.0", license: "MIT", source: "registry" },
  ];

  it("collects deterministic unique npm and cargo components", () => {
    const components = collectComponents(packageLock, cargoPackages);
    expect(components.map((item) => item.purl)).toEqual([
      "pkg:cargo/rusqlite@0.38.0",
      "pkg:npm/%40scope%2Ftool@1.2.3",
      "pkg:npm/react@19.0.0",
    ]);
    expect(new Set(components.map((item) => item.spdxId)).size).toBe(3);
  });

  it("builds SPDX 2.3 with declared licenses and dependency relationships", () => {
    const components = collectComponents(packageLock, cargoPackages);
    const spdx = buildSpdx({ name: "sorttrack", version: "0.1.0", components, lockDigest: "abc", created: "1970-01-01T00:00:00Z" });
    expect(spdx.spdxVersion).toBe("SPDX-2.3");
    expect(spdx.packages).toHaveLength(4);
    expect(spdx.relationships.filter((item) => item.relationshipType === "DEPENDS_ON")).toHaveLength(3);
    expect(JSON.stringify(spdx)).not.toContain("undefined");
  });

  it("quotes license inventory cells and hashes bytes reproducibly", () => {
    const components = collectComponents(packageLock, cargoPackages);
    expect(licenseCsv([{ ...components[0], name: 'crate,"one"' }])).toContain('"crate,""one"""');
    expect(sha256Buffer("SortTrack")).toBe("8fe85759cd7283cd8079e43076285b99f28d01299809d6f36d26374c31225aa0");
  });
});
