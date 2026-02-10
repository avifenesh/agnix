#!/usr/bin/env bash
# Verify that locale files in each crate match the root locales/ directory.
# The i18n!() macro loads from crate-local locales/, so they must stay in sync
# with the canonical copies under the workspace root.

set -euo pipefail

ROOT_LOCALES="locales"
CRATES=("crates/agnix-core" "crates/agnix-cli" "crates/agnix-lsp")
LOCALE_FILES=("en.yml" "es.yml" "zh-CN.yml")

errors=0

for crate in "${CRATES[@]}"; do
  crate_locales="${crate}/locales"

  if [ ! -d "$crate_locales" ]; then
    echo "FAIL: ${crate_locales}/ directory missing"
    errors=$((errors + 1))
    continue
  fi

  for file in "${LOCALE_FILES[@]}"; do
    root_file="${ROOT_LOCALES}/${file}"
    crate_file="${crate_locales}/${file}"

    if [ ! -f "$crate_file" ]; then
      echo "FAIL: ${crate_file} missing (expected copy of ${root_file})"
      errors=$((errors + 1))
    elif ! diff -q "$root_file" "$crate_file" > /dev/null 2>&1; then
      echo "FAIL: ${crate_file} differs from ${root_file}"
      diff --unified=3 "$root_file" "$crate_file" || true
      errors=$((errors + 1))
    fi
  done
done

if [ "$errors" -gt 0 ]; then
  echo ""
  echo "${errors} locale file(s) out of sync."
  echo "Run: cp locales/*.yml crates/agnix-{core,cli,lsp}/locales/"
  exit 1
fi

echo "All locale files in sync."
