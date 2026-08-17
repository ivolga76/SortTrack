export type CopyDecision = "copy" | "skip-identical" | "resolve-conflict" | "invalid";

export interface CopySource {
  trackId: string;
  sourcePath: string;
  filename: string;
  size: number;
  sha256?: string;
}

export interface DestinationEntry {
  path: string;
  size: number;
  sha256?: string;
}

export interface CopyPlanItem extends CopySource {
  destinationPath: string;
  decision: CopyDecision;
  reason: string;
}

const windowsReserved = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i;
const invalidWindowsChars = /[<>:"/\\|?*\u0000-\u001F]/;

export function isSafeWindowsFilename(filename: string): boolean {
  return (
    filename.length > 0 &&
    !filename.endsWith(".") &&
    !filename.endsWith(" ") &&
    !windowsReserved.test(filename) &&
    !invalidWindowsChars.test(filename)
  );
}

function pathKey(path: string): string {
  return path.replaceAll("/", "\\").toLocaleLowerCase("en-US");
}

export function joinWindowsPath(directory: string, filename: string): string {
  return `${directory.replace(/[\\/]+$/, "")}\\${filename}`;
}

export function planCopies(
  sources: CopySource[],
  destinationDirectory: string,
  existing: DestinationEntry[],
): CopyPlanItem[] {
  const existingByPath = new Map(existing.map((entry) => [pathKey(entry.path), entry]));

  return sources.map((source) => {
    const destinationPath = joinWindowsPath(destinationDirectory, source.filename);
    if (!isSafeWindowsFilename(source.filename)) {
      return {
        ...source,
        destinationPath,
        decision: "invalid",
        reason: "Недопустимое имя файла Windows",
      };
    }

    const destination = existingByPath.get(pathKey(destinationPath));
    if (!destination) {
      return { ...source, destinationPath, decision: "copy", reason: "Готово к копированию" };
    }

    if (source.sha256 && destination.sha256 && source.sha256 === destination.sha256) {
      return {
        ...source,
        destinationPath,
        decision: "skip-identical",
        reason: "Идентичный файл уже существует",
      };
    }

    return {
      ...source,
      destinationPath,
      decision: "resolve-conflict",
      reason: "Файл с таким именем уже существует",
    };
  });
}
