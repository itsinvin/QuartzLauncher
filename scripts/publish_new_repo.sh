#!/bin/bash
# Publish Quartz Launcher to its own GitHub repository.
# Run this locally with your GitHub credentials (gh auth login).
set -euo pipefail

REPO="${1:-itsinvin/QuartzLauncher}"
BRANCH="${2:-main}"

if ! command -v gh >/dev/null; then
    echo "GitHub CLI (gh) is required. Install from https://cli.github.com/"
    exit 1
fi

if ! gh repo view "$REPO" >/dev/null 2>&1; then
    echo "Creating https://github.com/$REPO ..."
    gh repo create "$REPO" \
        --public \
        --description "Quartz Launcher — enhanced PandoraLauncher fork. Native Rust/GPUI Minecraft launcher." \
        --source=. \
        --remote=origin \
        --push
    echo "Done. Repository: https://github.com/$REPO"
    exit 0
fi

echo "Repository exists. Pushing $BRANCH to https://github.com/$REPO ..."
git remote remove origin 2>/dev/null || true
git remote add origin "https://github.com/$REPO.git"
git push -u origin "$BRANCH"
git push origin --tags 2>/dev/null || true
echo "Done. Repository: https://github.com/$REPO"
