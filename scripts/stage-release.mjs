import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { version } = JSON.parse(readFileSync(resolve(projectRoot, "package.json"), "utf8"));
const bundleRoot = resolve(projectRoot, "src-tauri", "target", "release", "bundle");
const releaseRoot = resolve(bundleRoot, "release");
const artifacts = [
  [`nsis/SortTrack_${version}_x64-setup.exe`, `SortTrack_${version}_x64-setup.exe`],
  [`portable/SortTrackPortable-${version}-x64.zip`, `SortTrackPortable-${version}-x64.zip`],
];

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

mkdirSync(releaseRoot, { recursive: true });
for (const [sourceName, targetName] of artifacts) {
  const source = resolve(bundleRoot, sourceName);
  const target = resolve(releaseRoot, targetName);
  copyFileSync(source, target);
  if (sha256(source) !== sha256(target)) throw new Error(`Release staging verification failed: ${targetName}`);
  console.log(`${targetName}: ${sha256(target)}`);
}
console.log(`Release directory: ${releaseRoot}`);
