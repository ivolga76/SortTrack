import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function parseChecksums(text) {
  const entries = new Map();
  for (const line of text.trim().split(/\r?\n/)) {
    const match = line.match(/^([a-f0-9]{64}) {2}([^/\\]+)$/);
    if (!match) throw new Error(`Invalid SHA256SUMS line: ${line}`);
    if (entries.has(match[2])) throw new Error(`Duplicate checksum entry: ${match[2]}`);
    entries.set(match[2], match[1]);
  }
  return entries;
}

export function validateSpdx(spdx, manifest) {
  if (spdx.spdxVersion !== "SPDX-2.3" || spdx.dataLicense !== "CC0-1.0") throw new Error("Invalid SPDX document header");
  const packages = spdx.packages ?? [];
  const ids = packages.map((item) => item.SPDXID);
  if (packages.length !== manifest.dependencies.total + 1) throw new Error("SPDX package count does not match release manifest");
  if (new Set(ids).size !== ids.length) throw new Error("SPDX package identifiers are not unique");
  if (packages.some((item) => !item.licenseDeclared || item.licenseDeclared === "NOASSERTION")) throw new Error("SPDX contains an undeclared license");
  if ((spdx.relationships ?? []).length !== packages.length) throw new Error("SPDX relationship count is incomplete");
}

export function verifyRelease(releaseDirectory) {
  const root = resolve(releaseDirectory);
  const { version } = JSON.parse(readFileSync(resolve(projectRoot, "package.json"), "utf8"));
  const installer = `SortTrack_${version}_x64-setup.exe`;
  const portable = `SortTrackPortable-${version}-x64.zip`;
  const spdxName = `SortTrack-${version}.spdx.json`;
  const required = [installer, portable, spdxName, "LICENSE", "THIRD_PARTY_LICENSES.csv", "release-manifest.json", "SHA256SUMS.txt"];
  for (const name of required) {
    if (!statSync(resolve(root, name)).isFile()) throw new Error(`Missing release file: ${name}`);
  }

  const checksums = parseChecksums(readFileSync(resolve(root, "SHA256SUMS.txt"), "utf8"));
  const checksummedFiles = required.filter((name) => name !== "SHA256SUMS.txt");
  if (checksums.size !== checksummedFiles.length) throw new Error("SHA256SUMS does not cover every release file exactly once");
  for (const name of checksummedFiles) {
    if (checksums.get(name) !== sha256(resolve(root, name))) throw new Error(`SHA-256 mismatch: ${name}`);
  }

  const manifest = JSON.parse(readFileSync(resolve(root, "release-manifest.json"), "utf8"));
  if (manifest.schema !== "sorttrack-release-manifest-1" || manifest.version !== version) throw new Error("Release manifest identity mismatch");
  if (manifest.dependencies.unknownLicense !== 0) throw new Error("Release contains dependencies without a declared license");
  const artifactNames = new Set(manifest.artifacts.map((item) => item.file));
  if (artifactNames.size !== 2 || !artifactNames.has(installer) || !artifactNames.has(portable)) throw new Error("Release manifest must contain installed and portable artifacts");
  for (const artifact of manifest.artifacts) {
    const path = resolve(root, artifact.file);
    if (artifact.size !== statSync(path).size || artifact.sha256 !== sha256(path)) throw new Error(`Release manifest mismatch: ${artifact.file}`);
  }

  const spdx = JSON.parse(readFileSync(resolve(root, spdxName), "utf8"));
  validateSpdx(spdx, manifest);
  const publicMetadata = JSON.stringify({ manifest, spdx });
  if (/[A-Za-z]:\\\\|\/(?:home|Users)\//.test(publicMetadata)) throw new Error("Public release metadata contains an absolute workstation path");

  const licenses = readFileSync(resolve(root, "THIRD_PARTY_LICENSES.csv"), "utf8").trim().split(/\r?\n/);
  if (licenses.length !== manifest.dependencies.total + 1 || licenses.some((line) => line.includes("NOASSERTION"))) throw new Error("License CSV coverage mismatch");
  if (sha256(resolve(root, "LICENSE")) !== sha256(resolve(projectRoot, "LICENSE"))) throw new Error("Project LICENSE mismatch");

  const archiveEntries = execFileSync("tar", ["-tf", resolve(root, portable)], { encoding: "utf8" }).trim().split(/\r?\n/).map((entry) => entry.replaceAll("\\", "/"));
  const portableRoot = `SortTrackPortable-${version}/`;
  for (const entry of [".sorttrack-portable", "data/", "README_PORTABLE.txt", "SortTrack.exe", "LICENSE", "THIRD_PARTY_NOTICES.md"]) {
    if (!archiveEntries.includes(`${portableRoot}${entry}`)) throw new Error(`Portable archive is missing ${entry}`);
  }
  if (archiveEntries.some((entry) => /sorttrack\.db(?:-|$)/i.test(basename(entry)))) throw new Error("Portable archive must not contain a user database");
  return { files: required.length, dependencies: manifest.dependencies.total, packages: spdx.packages.length };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const directory = process.argv[2] ?? resolve(projectRoot, "src-tauri", "target", "release", "bundle", "release");
  const report = verifyRelease(directory);
  console.log(`Release verified: ${report.files} files, ${report.dependencies} dependencies, ${report.packages} SPDX packages`);
}
