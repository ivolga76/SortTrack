import fs from "node:fs";
import path from "node:path";

export const CURRENT_SCHEMA_VERSION = 13;

export function migrationFiles(migrationsDirectory) {
  return fs.readdirSync(migrationsDirectory)
    .filter((name) => /^\d{4}_[a-z0-9_]+\.sql$/.test(name))
    .sort();
}

export function applyCurrentMigrations(database, migrationsDirectory) {
  const files = migrationFiles(migrationsDirectory);
  if (files.length !== CURRENT_SCHEMA_VERSION) {
    throw new Error(`Expected ${CURRENT_SCHEMA_VERSION} migrations, found ${files.length}`);
  }
  for (const file of files) {
    database.exec(fs.readFileSync(path.join(migrationsDirectory, file), "utf8"));
  }
  const version = Number(database.prepare("PRAGMA user_version").get().user_version);
  if (version !== CURRENT_SCHEMA_VERSION) {
    throw new Error(`Expected schema v${CURRENT_SCHEMA_VERSION}, got v${version}`);
  }
  return version;
}

export function assertHealthyDatabase(database) {
  const integrity = database.prepare("PRAGMA integrity_check").get().integrity_check;
  if (integrity !== "ok") throw new Error(`SQLite integrity check failed: ${integrity}`);
  const foreignKeyErrors = database.prepare("SELECT COUNT(*) AS count FROM pragma_foreign_key_check").get().count;
  if (Number(foreignKeyErrors) !== 0) throw new Error(`SQLite foreign key check found ${foreignKeyErrors} errors`);
}
