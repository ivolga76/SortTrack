# Third-party notices

The authoritative, version-pinned dependency inventory is generated for every Windows release by:

```text
npm run release:metadata
```

The release directory contains:

- `SortTrack-<version>.spdx.json` — SPDX 2.3 SBOM;
- `THIRD_PARTY_LICENSES.csv` — dependency, exact version, ecosystem, declared license and Package URL;
- `release-manifest.json` — artifact sizes, hashes and license coverage;
- `SHA256SUMS.txt` — SHA-256 for the installer and generated metadata.

Generation is offline and restricted to the `x86_64-pc-windows-msvc` Cargo dependency graph. An absent license is emitted as `NOASSERTION`, never guessed; the release manifest reports its count.

Initial direct dependencies:

- React — MIT
- Vite — MIT
- TypeScript — Apache-2.0
- Vitest — MIT
- Tauri — Apache-2.0 OR MIT
- rusqlite — MIT
- Lofty 0.24 — Apache-2.0 OR MIT
- Symphonia 0.6 — MPL-2.0
- RustFFT 6.4 — Apache-2.0 OR MIT
- walkdir — Unlicense OR MIT
- sha2 0.11 — Apache-2.0 OR MIT
- fs2 0.4 — Apache-2.0 OR MIT
- windows-sys 0.61 — Apache-2.0 OR MIT

Bundled audio binaries and ML models must not be added before their exact versions, build configuration, checksums and licenses are included in this release inventory.
