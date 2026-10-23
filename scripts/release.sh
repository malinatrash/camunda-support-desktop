#!/usr/bin/env bash
set -euo pipefail

tag="${1:-}"
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Использование: ./scripts/release.sh v2.0.0" >&2
  exit 1
fi

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

if [[ "$(git branch --show-current)" != "main" ]]; then
  echo "Релиз можно создавать только из ветки main." >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Перед релизом рабочее дерево должно быть чистым." >&2
  exit 1
fi

git fetch origin main --tags

if ! git merge-base --is-ancestor origin/main HEAD; then
  echo "Локальная main расходится с origin/main. Сначала синхронизируйте историю." >&2
  exit 1
fi

if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null || \
   git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "Тег $tag уже существует." >&2
  exit 1
fi

pnpm install --frozen-lockfile
pnpm check
pnpm test
(cd src-tauri && cargo test)

node scripts/set-version.mjs "$tag"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "release: ${tag#v}"
git tag -a "$tag" -m "Camunda Support ${tag#v}"
git push --atomic origin main "$tag"

echo "Релиз $tag запущен. GitHub Actions соберёт Windows, macOS и Ubuntu пакеты."
