#!/usr/bin/env bash
# Publish site/ to the gh-pages branch that GitHub Pages serves.
# Usage: bash site/publish.sh [--dry-run]
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

branch=gh-pages
remote=origin

if [[ "${1:-}" == "--dry-run" ]]; then
  out=tmp/gh-pages
else
  out=$(mktemp -d)
fi

assemble() {
  mkdir -p "$1/assets" "$1/docs"
  cp -R site/. "$1/"
  rm -f "$1/publish.sh"
  cp -R assets/brand "$1/assets/brand"
  cp -R docs/assets "$1/docs/assets"
  touch "$1/.nojekyll"
}

if [[ "${1:-}" == "--dry-run" ]]; then
  rm -rf "$out"
  assemble "$out"
  echo "Assembled $out"
  exit 0
fi

assemble "$out"
git -C "$out" init -q -b "$branch"
git -C "$out" add -A
git -C "$out" -c user.name="$(git config user.name)" -c user.email="$(git config user.email)" \
  commit -q -m "Publish site from $(git rev-parse --short HEAD)"
git -C "$out" push -f "$(git remote get-url "$remote")" "$branch"
rm -rf "$out"
echo "Published to $remote/$branch"
