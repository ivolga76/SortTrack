import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, "..");

export function extractFrontendCommands(source) {
  return [...source.matchAll(/invoke(?:<[^>]+>)?\(\s*["']([a-z0-9_]+)["']/g)]
    .map((match) => match[1])
    .filter((value, index, values) => values.indexOf(value) === index)
    .sort();
}

export function extractBackendCommands(source) {
  const match = source.match(/generate_handler!\[(?<body>[\s\S]*?)\]\)/);
  if (!match?.groups?.body) throw new Error("Tauri generate_handler command list was not found");
  return match.groups.body
    .split(",")
    .map((value) => value.trim())
    .filter((value) => /^[a-z0-9_]+$/.test(value))
    .filter((value, index, values) => values.indexOf(value) === index)
    .sort();
}

export function buildIpcInventory(frontendSource, backendSource) {
  const frontendCommands = extractFrontendCommands(frontendSource);
  const backendCommands = extractBackendCommands(backendSource);
  const frontendSet = new Set(frontendCommands);
  const backendSet = new Set(backendCommands);
  return {
    schema: "sorttrack-ipc-command-inventory-1",
    frontendCount: frontendCommands.length,
    backendCount: backendCommands.length,
    frontendCommands,
    backendCommands,
    frontendOnly: frontendCommands.filter((command) => !backendSet.has(command)),
    backendOnly: backendCommands.filter((command) => !frontendSet.has(command)),
  };
}

export function currentIpcInventory(root = projectRoot) {
  const frontendSource = fs.readFileSync(path.join(root, "src", "adapters", "tauri", "TauriLibraryGateway.ts"), "utf8");
  const backendSource = fs.readFileSync(path.join(root, "src-tauri", "src", "lib.rs"), "utf8");
  return buildIpcInventory(frontendSource, backendSource);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const inventory = currentIpcInventory();
  const requestedSnapshot = process.argv[2];
  if (!requestedSnapshot) {
    process.stdout.write(`${JSON.stringify(inventory, null, 2)}\n`);
  } else {
    const snapshotPath = path.resolve(projectRoot, requestedSnapshot);
    const expected = JSON.parse(fs.readFileSync(snapshotPath, "utf8"));
    if (JSON.stringify(inventory) !== JSON.stringify(expected)) {
      throw new Error(`IPC inventory differs from ${snapshotPath}`);
    }
    process.stdout.write(`IPC inventory matches ${snapshotPath}\n`);
  }
}
