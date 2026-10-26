#!/bin/bash
# Deploy the committed site (HEAD) to Cloudflare Pages project "hashawn".
# Bench data, sources and dev tooling stay out; functions/ (the Access check) goes along.
set -euo pipefail
cd "$(dirname "$0")"
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
git archive HEAD | tar -x -C "$out"
rm -rf "$out"/{functions,ncz/bench,slm/src,deploy.sh,dev.py} "$out"/slm/*.py "$out"/slm/Cargo.*
find "$out" -name '*.py' -delete
npx -y wrangler@latest pages deploy "$out" --project-name hashawn --branch main --commit-dirty=true
