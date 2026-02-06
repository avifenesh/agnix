#!/usr/bin/env bash
# Sync version from Cargo.toml to ALL package manifests.
# Run before tagging a release or as part of CI.
#
# Accepts optional argument to override version:
#   ./sync-versions.sh 0.9.0

set -euo pipefail

if [ -n "${1:-}" ]; then
  VERSION="$1"
else
  VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
fi

if [ -z "$VERSION" ]; then
  echo "Error: Could not extract version from Cargo.toml"
  exit 1
fi

echo "Syncing all manifests to version: $VERSION"

# VS Code extension
if [ -f editors/vscode/package.json ]; then
  sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" editors/vscode/package.json
  rm -f editors/vscode/package.json.bak
  echo "  editors/vscode/package.json -> $VERSION"
fi

# VS Code package-lock.json
if [ -f editors/vscode/package-lock.json ]; then
  sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" editors/vscode/package-lock.json
  rm -f editors/vscode/package-lock.json.bak
  echo "  editors/vscode/package-lock.json -> $VERSION"
fi

# npm package
if [ -f npm/package.json ]; then
  sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" npm/package.json
  rm -f npm/package.json.bak
  echo "  npm/package.json -> $VERSION"
fi

# JetBrains plugin
if [ -f editors/jetbrains/gradle.properties ]; then
  sed -i.bak "s/pluginVersion = .*/pluginVersion = $VERSION/" editors/jetbrains/gradle.properties
  rm -f editors/jetbrains/gradle.properties.bak
  echo "  editors/jetbrains/gradle.properties -> $VERSION"
fi

# Zed extension
if [ -f editors/zed/extension.toml ]; then
  sed -i.bak "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" editors/zed/extension.toml
  rm -f editors/zed/extension.toml.bak
  echo "  editors/zed/extension.toml -> $VERSION"
fi

echo "Done. All manifests synced to $VERSION"
