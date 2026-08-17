declare const expectedStateScope: unique symbol;

export type ExpectedStateScope = "playlist-removal" | "playlist-export" | "metadata-file" | "relink" | "diagnostics";
export type ExpectedStateToken<Scope extends ExpectedStateScope> = string & { readonly [expectedStateScope]: Scope };

export function expectedStateToken<Scope extends ExpectedStateScope>(value: string): ExpectedStateToken<Scope> {
  return value as ExpectedStateToken<Scope>;
}

export type PlaylistRemovalState = ExpectedStateToken<"playlist-removal">;
export type PlaylistExportState = ExpectedStateToken<"playlist-export">;
export type MetadataFileState = ExpectedStateToken<"metadata-file">;
export type RelinkState = ExpectedStateToken<"relink">;
export type DiagnosticsState = ExpectedStateToken<"diagnostics">;
