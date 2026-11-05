#!/usr/bin/env node
// Записывает версию релиза (из тега vX.Y.Z или аргумента X.Y.Z) в версию workspace и Cargo.lock.
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const raw = (process.argv[2] ?? process.env.GITHUB_REF_NAME ?? "").trim();
const match = raw.match(/^v?(\d+\.\d+\.\d+)$/);
if (!match) {
  console.error(`Использование: node scripts/set-version.mjs v1.2.3 (получено: "${raw}")`);
  process.exit(1);
}
const version = match[1];

function rewrite(file, transform) {
  const path = resolve(root, file);
  const before = readFileSync(path, "utf8");
  const after = transform(before);
  if (after !== before) writeFileSync(path, after);
  console.log(`${file}: ${version}`);
}

rewrite("Cargo.toml", (text) =>
  text.replace(/(\[workspace\.package\]\nversion = ")[^"]+(")/, `$1${version}$2`),
);
rewrite("Cargo.lock", (text) =>
  text.replace(/(\[\[package\]\]\nname = "camunda-(?:core|support)"\nversion = ")[^"]+(")/g, `$1${version}$2`),
);
