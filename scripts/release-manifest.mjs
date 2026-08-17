import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

export function sha256Buffer(value) {
  return createHash("sha256").update(value).digest("hex");
}

export function sha256File(path) {
  return sha256Buffer(readFileSync(path));
}

function spdxId(purl) {
  return `SPDXRef-Package-${sha256Buffer(purl).slice(0, 20)}`;
}

function npmName(path, descriptor) {
  if (descriptor.name) return descriptor.name;
  const marker = "node_modules/";
  const index = path.lastIndexOf(marker);
  return index < 0 ? path : path.slice(index + marker.length);
}

function component(ecosystem, name, version, license) {
  const encodedName = ecosystem === "npm" ? name.replace(/^@/, "%40").replace("/", "%2F") : name;
  const purl = `pkg:${ecosystem}/${encodedName}@${version}`;
  return { ecosystem, name, version, license: typeof license === "string" && license.trim() ? license : "NOASSERTION", purl, spdxId: spdxId(purl) };
}

export function collectComponents(packageLock, cargoPackages) {
  const npm = Object.entries(packageLock.packages ?? {})
    .filter(([path, descriptor]) => path !== "" && descriptor.version)
    .map(([path, descriptor]) => component("npm", npmName(path, descriptor), descriptor.version, descriptor.license));
  const cargo = (cargoPackages ?? [])
    .filter((item) => !(item.name === packageLock.name && item.source === null))
    .map((item) => component("cargo", item.name, item.version, item.license));
  return [...new Map([...npm, ...cargo].map((item) => [item.purl, item])).values()]
    .sort((left, right) => left.purl.localeCompare(right.purl, "en"));
}

function sourceDate() {
  const seconds = Number.parseInt(process.env.SOURCE_DATE_EPOCH ?? "0", 10);
  if (!Number.isFinite(seconds) || seconds < 0) throw new Error("SOURCE_DATE_EPOCH must be a non-negative integer");
  return new Date(seconds * 1000).toISOString().replace(".000Z", "Z");
}

export function buildSpdx({ name, version, components, lockDigest, created = sourceDate() }) {
  const rootPurl = `pkg:generic/${name}@${version}`;
  const rootId = spdxId(rootPurl);
  return {
    spdxVersion: "SPDX-2.3",
    dataLicense: "CC0-1.0",
    SPDXID: "SPDXRef-DOCUMENT",
    name: `${name}-${version}`,
    documentNamespace: `https://sorttrack.local/spdx/${name}/${version}/${lockDigest}`,
    creationInfo: { created, creators: ["Tool: SortTrack release-manifest"] },
    packages: [
      {
        name,
        versionInfo: version,
        SPDXID: rootId,
        downloadLocation: "NOASSERTION",
        filesAnalyzed: false,
        licenseConcluded: "AGPL-3.0-only",
        licenseDeclared: "AGPL-3.0-only",
        externalRefs: [{ referenceCategory: "PACKAGE-MANAGER", referenceType: "purl", referenceLocator: rootPurl }],
      },
      ...components.map((item) => ({
        name: item.name,
        versionInfo: item.version,
        SPDXID: item.spdxId,
        downloadLocation: "NOASSERTION",
        filesAnalyzed: false,
        licenseConcluded: "NOASSERTION",
        licenseDeclared: item.license,
        externalRefs: [{ referenceCategory: "PACKAGE-MANAGER", referenceType: "purl", referenceLocator: item.purl }],
      })),
    ],
    relationships: [
      { spdxElementId: "SPDXRef-DOCUMENT", relationshipType: "DESCRIBES", relatedSpdxElement: rootId },
      ...components.map((item) => ({ spdxElementId: rootId, relationshipType: "DEPENDS_ON", relatedSpdxElement: item.spdxId })),
    ],
  };
}

function csvCell(value) {
  const text = String(value);
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

export function licenseCsv(components) {
  return ["ecosystem,name,version,license,purl", ...components.map((item) =>
    [item.ecosystem, item.name, item.version, item.license, item.purl].map(csvCell).join(","),
  )].join("\n") + "\n";
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

export function generateReleaseManifest(artifactPaths, outputDirectory) {
  if (!artifactPaths.length) throw new Error("At least one release artifact is required");
  const packageLockPath = resolve(projectRoot, "package-lock.json");
  const cargoLockPath = resolve(projectRoot, "src-tauri", "Cargo.lock");
  const packageLock = JSON.parse(readFileSync(packageLockPath, "utf8"));
  const cargoMetadata = JSON.parse(execFileSync("cargo", [
    "metadata", "--locked", "--offline", "--format-version", "1",
    "--filter-platform", process.env.SORTTRACK_RELEASE_TARGET ?? "x86_64-pc-windows-msvc",
    "--manifest-path", resolve(projectRoot, "src-tauri", "Cargo.toml"),
  ], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 }));
  const cargoPackages = cargoMetadata.packages;
  const components = collectComponents(packageLock, cargoPackages);
  const lockDigest = sha256Buffer(Buffer.concat([readFileSync(packageLockPath), readFileSync(cargoLockPath)]));
  const output = resolve(outputDirectory);
  mkdirSync(output, { recursive: true });

  const spdxPath = resolve(output, `SortTrack-${packageLock.version}.spdx.json`);
  const licensesPath = resolve(output, "THIRD_PARTY_LICENSES.csv");
  const projectLicensePath = resolve(output, "LICENSE");
  const manifestPath = resolve(output, "release-manifest.json");
  const sumsPath = resolve(output, "SHA256SUMS.txt");
  writeJson(spdxPath, buildSpdx({ name: packageLock.name, version: packageLock.version, components, lockDigest }));
  writeFileSync(licensesPath, licenseCsv(components), "utf8");
  copyFileSync(resolve(projectRoot, "LICENSE"), projectLicensePath);

  const artifacts = artifactPaths.map((path) => {
    const absolutePath = resolve(projectRoot, path);
    const size = statSync(absolutePath).size;
    return { file: basename(absolutePath), size, sha256: sha256File(absolutePath) };
  });
  const generated = [spdxPath, licensesPath, projectLicensePath].map((path) => ({ file: basename(path), size: statSync(path).size, sha256: sha256File(path) }));
  writeJson(manifestPath, {
    schema: "sorttrack-release-manifest-1",
    product: packageLock.name,
    version: packageLock.version,
    dependencies: {
      total: components.length,
      npm: components.filter((item) => item.ecosystem === "npm").length,
      cargo: components.filter((item) => item.ecosystem === "cargo").length,
      declaredLicense: components.filter((item) => item.license !== "NOASSERTION").length,
      unknownLicense: components.filter((item) => item.license === "NOASSERTION").length,
    },
    artifacts,
    generated,
  });
  const checksummed = [...artifacts, ...generated, { file: basename(manifestPath), sha256: sha256File(manifestPath) }]
    .sort((left, right) => left.file.localeCompare(right.file, "en"));
  writeFileSync(sumsPath, checksummed.map((item) => `${item.sha256}  ${item.file}`).join("\n") + "\n", "utf8");
  return { output, artifacts, components: components.length, spdxPath, licensesPath, manifestPath, sumsPath };
}

function parseArguments(args) {
  const outputIndex = args.indexOf("--output");
  let output = resolve(projectRoot, "src-tauri", "target", "release", "bundle", "nsis");
  if (outputIndex >= 0) {
    if (!args[outputIndex + 1]) throw new Error("--output requires a directory");
    output = args[outputIndex + 1];
    args = [...args.slice(0, outputIndex), ...args.slice(outputIndex + 2)];
  }
  return { output, artifacts: args };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { output, artifacts } = parseArguments(process.argv.slice(2));
  const report = generateReleaseManifest(artifacts, output);
  console.log(`Release metadata created in ${report.output}`);
  console.log(`Dependencies: ${report.components}; artifacts: ${report.artifacts.length}`);
}
