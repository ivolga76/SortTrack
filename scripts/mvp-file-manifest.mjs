import { createHash } from "node:crypto";
import {
  closeSync,
  lstatSync,
  mkdirSync,
  openSync,
  readFileSync,
  readSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, extname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const AUDIO_EXTENSIONS = new Set([".mp3", ".flac"]);
const READ_BUFFER_SIZE = 1024 * 1024;

function hashFile(path) {
  const hash = createHash("sha256");
  const buffer = Buffer.allocUnsafe(READ_BUFFER_SIZE);
  const descriptor = openSync(path, "r");
  try {
    for (;;) {
      const bytesRead = readSync(descriptor, buffer, 0, buffer.length, null);
      if (bytesRead === 0) break;
      hash.update(buffer.subarray(0, bytesRead));
    }
  } finally {
    closeSync(descriptor);
  }
  return hash.digest("hex");
}

function audioFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = resolve(directory, entry.name);
      const metadata = lstatSync(path);
      if (metadata.isSymbolicLink()) throw new Error(`Symbolic links are not allowed in an acceptance corpus: ${path}`);
      if (metadata.isDirectory()) visit(path);
      else if (metadata.isFile() && AUDIO_EXTENSIONS.has(extname(entry.name).toLowerCase())) files.push(path);
    }
  };
  visit(root);
  return files.sort((left, right) => left.localeCompare(right, "en", { sensitivity: "base" }));
}

export function buildAudioManifest(rootPath, generatedAt = new Date().toISOString()) {
  const root = resolve(rootPath);
  if (!statSync(root).isDirectory()) throw new Error(`Corpus root is not a directory: ${root}`);
  const seen = new Set();
  const files = audioFiles(root).map((path) => {
    const before = statSync(path);
    const sha256 = hashFile(path);
    const after = statSync(path);
    if (before.size !== after.size || before.mtimeMs !== after.mtimeMs) throw new Error(`File changed while hashing: ${path}`);
    const relativePath = relative(root, path).replaceAll("\\", "/");
    const identity = relativePath.toLocaleLowerCase("en-US");
    if (seen.has(identity)) throw new Error(`Case-insensitive path collision: ${relativePath}`);
    seen.add(identity);
    return {
      relativePath,
      extension: extname(path).slice(1).toLowerCase(),
      size: after.size,
      modifiedMs: Math.trunc(after.mtimeMs),
      sha256,
    };
  });
  return {
    schema: "sorttrack-mvp-audio-manifest-1",
    generatedAt,
    root,
    hashAlgorithm: "SHA-256",
    fileCount: files.length,
    totalBytes: files.reduce((total, file) => total + file.size, 0),
    formatCounts: {
      mp3: files.filter((file) => file.extension === "mp3").length,
      flac: files.filter((file) => file.extension === "flac").length,
    },
    files,
  };
}

export function compareAudioManifests(expected, actual) {
  if (expected.schema !== "sorttrack-mvp-audio-manifest-1" || actual.schema !== "sorttrack-mvp-audio-manifest-1") {
    throw new Error("Unsupported acceptance manifest schema");
  }
  const expectedFiles = new Map(expected.files.map((file) => [file.relativePath.toLocaleLowerCase("en-US"), file]));
  const actualFiles = new Map(actual.files.map((file) => [file.relativePath.toLocaleLowerCase("en-US"), file]));
  const missing = [];
  const added = [];
  const changed = [];
  for (const [identity, file] of expectedFiles) {
    const current = actualFiles.get(identity);
    if (!current) missing.push(file.relativePath);
    else if (file.size !== current.size || file.sha256 !== current.sha256) changed.push(file.relativePath);
  }
  for (const [identity, file] of actualFiles) {
    if (!expectedFiles.has(identity)) added.push(file.relativePath);
  }
  return {
    unchanged: missing.length === 0 && added.length === 0 && changed.length === 0,
    expectedFiles: expected.files.length,
    actualFiles: actual.files.length,
    missing,
    added,
    changed,
  };
}

export function parseManifestArguments(args) {
  const options = { root: null, output: null, compare: null };
  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index];
    const value = args[index + 1];
    if (!value) throw new Error(`Missing value for ${flag ?? "argument"}`);
    if (flag === "--root") options.root = value;
    else if (flag === "--output") options.output = value;
    else if (flag === "--compare") options.compare = value;
    else throw new Error(`Unknown argument: ${flag}`);
  }
  if (!options.root) throw new Error("--root is required");
  if (Boolean(options.output) === Boolean(options.compare)) throw new Error("Specify exactly one of --output or --compare");
  return options;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const options = parseManifestArguments(process.argv.slice(2));
    const actual = buildAudioManifest(options.root);
    if (options.output) {
      const output = resolve(options.output);
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, `${JSON.stringify(actual, null, 2)}\n`, { encoding: "utf8", flag: "wx" });
      console.log(`Manifest created: ${output}`);
      console.log(`Files: ${actual.fileCount}; MP3: ${actual.formatCounts.mp3}; FLAC: ${actual.formatCounts.flac}`);
    } else {
      const expected = JSON.parse(readFileSync(resolve(options.compare), "utf8"));
      const report = compareAudioManifests(expected, actual);
      console.log(JSON.stringify(report, null, 2));
      if (!report.unchanged) process.exitCode = 2;
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
