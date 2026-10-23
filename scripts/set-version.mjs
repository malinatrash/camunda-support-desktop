#!/usr/bin/env node
// Записывает версию релиза (из тега vX.Y.Z или аргумента X.Y.Z) в package.json, tauri.conf.json и Cargo.toml.
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

rewrite("package.json", (text) => {
  const json = JSON.parse(text);
  json.version = version;
  return `${JSON.stringify(json, null, 2)}\n`;
});
rewrite("src-tauri/tauri.conf.json", (text) => {
  const json = JSON.parse(text);
  json.version = version;
  return `${JSON.stringify(json, null, 2)}\n`;
});
rewrite("src-tauri/Cargo.toml", (text) =>
  text.replace(/^version = "[^"]+"/m, `version = "${version}"`),
);
rewrite("src-tauri/Cargo.lock", (text) =>
  text.replace(/(\[\[package\]\]\nname = "camunda-support"\nversion = ")[^"]+(")/, `$1${version}$2`),
);
