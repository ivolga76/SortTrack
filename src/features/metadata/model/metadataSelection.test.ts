import { describe, expect, it } from "vitest";
import type { Track } from "../../../domain/models";
import { resolveMetadataSelection } from "./metadataSelection";

const track = (id: string) => ({ id } as Track);

describe("metadata selection", () => {
  it("uses bulk mode for more than one checked track", () => {
    expect(resolveMetadataSelection(new Set(["a", "b"]), [track("a"), track("b")], null)).toEqual({ mode: "bulk", trackIds: ["a", "b"] });
  });

  it("prefers the sole checked track over the focused row", () => {
    expect(resolveMetadataSelection(new Set(["b"]), [track("a"), track("b")], track("a"))).toEqual({ mode: "single", track: track("b") });
  });

  it("falls back to the focused row and rejects stale checked ids", () => {
    expect(resolveMetadataSelection(new Set(), [track("a")], track("a"))).toEqual({ mode: "single", track: track("a") });
    expect(resolveMetadataSelection(new Set(["missing"]), [track("a")], track("a"))).toEqual({ mode: "none" });
  });
});
