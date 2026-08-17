import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packageJson = JSON.parse(readFileSync(resolve(projectRoot, "package.json"), "utf8"));
const bundleRoot = resolve(projectRoot, "src-tauri", "target", "release", "bundle", "portable");
const folderName = `SortTrackPortable-${packageJson.version}`;
const portableRoot = resolve(bundleRoot, folderName);
const executableSource = resolve(projectRoot, "src-tauri", "target", "release", "sorttrack.exe");
const executableTarget = resolve(portableRoot, "SortTrack.exe");
const archive = resolve(bundleRoot, `${folderName}-x64.zip`);

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

mkdirSync(resolve(portableRoot, "data"), { recursive: true });
copyFileSync(executableSource, executableTarget);
copyFileSync(resolve(projectRoot, "LICENSE"), resolve(portableRoot, "LICENSE"));
copyFileSync(resolve(projectRoot, "THIRD_PARTY_NOTICES.md"), resolve(portableRoot, "THIRD_PARTY_NOTICES.md"));
copyFileSync(resolve(projectRoot, "docs", "README_PORTABLE.txt"), resolve(portableRoot, "README_PORTABLE.txt"));
writeFileSync(resolve(portableRoot, ".sorttrack-portable"), "SortTrack portable storage profile v1\n", "utf8");
if (sha256(executableSource) !== sha256(executableTarget)) throw new Error("Portable executable copy verification failed");

rmSync(archive, { force: true });
const packed = spawnSync("tar", ["-a", "-c", "-f", archive, "-C", bundleRoot, folderName], { encoding: "utf8" });
if (packed.status !== 0) throw new Error(`Portable archive failed: ${packed.stderr || packed.stdout}`);
if (!statSync(archive).isFile() || statSync(archive).size === 0) throw new Error("Portable archive is empty");

console.log(`Portable folder: ${portableRoot}`);
console.log(`Portable archive: ${archive}`);
console.log(`Executable SHA-256: ${sha256(executableTarget)}`);
console.log(`Archive SHA-256: ${sha256(archive)}`);
console.log(`Archive file: ${basename(archive)}`);
