import { describe, expect, it } from "vitest";
import { expectedStateToken } from "./stateTokens";

describe("expected state tokens", () => {
  it("remain plain strings on the IPC wire", () => {
    const token = expectedStateToken<"metadata-file">("opaque-file-state");
    expect(token).toBe("opaque-file-state");
    expect(JSON.stringify({ expectedFileState: token })).toBe('{"expectedFileState":"opaque-file-state"}');
  });
});
