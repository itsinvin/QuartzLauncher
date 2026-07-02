#!/bin/bash
set -euo pipefail

if ! git diff --quiet -- README.md; then
  git config user.name "github-actions[bot]"
  git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
  git add README.md
  git commit -m "docs: update README build info"
  git push
fi
