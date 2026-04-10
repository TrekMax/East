#!/usr/bin/env bash
# Bump all crate versions in the workspace to the specified version.
# Usage: ./scripts/bump-version.sh 0.2.1
#
# This updates:
# - [package] version in all member crates
# - Inter-crate dependency version references
#
# After running, commit and tag:
#   git add -A && git commit -m "chore: bump all crate versions to $VERSION"
#   git tag "v$VERSION"
#   git push origin main --tags

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.1"
    exit 1
fi

NEW_VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Validate version format (semver)
if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
    echo "Error: version must be semver (e.g. 0.2.1, 1.0.0-beta.1)"
    exit 1
fi

echo "Bumping all crate versions to $NEW_VERSION..."

# Get current version from any crate (they should all be the same)
CURRENT_VERSION=$(grep -m1 '^version = ' "$ROOT_DIR/crates/east-manifest/Cargo.toml" | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $CURRENT_VERSION"
echo "New version: $NEW_VERSION"

if [ "$CURRENT_VERSION" = "$NEW_VERSION" ]; then
    echo "Already at version $NEW_VERSION, nothing to do."
    exit 0
fi

# Update [package] version in all member crates
for toml in "$ROOT_DIR"/crates/*/Cargo.toml; do
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$toml"
done

# Update inter-crate dependency version references
# Pattern: east-xxx = { version = "OLD", path = "..." }
for toml in "$ROOT_DIR"/crates/*/Cargo.toml; do
    sed -i "s/version = \"$CURRENT_VERSION\", path/version = \"$NEW_VERSION\", path/g" "$toml"
done

echo "Done. Updated files:"
git -C "$ROOT_DIR" diff --name-only 2>/dev/null || find "$ROOT_DIR/crates" -name "Cargo.toml" -newer "$0"

echo ""
echo "Next steps:"
echo "  cd $ROOT_DIR"
echo "  cargo check --workspace"
echo "  git add -A && git commit -m 'chore: bump all crate versions to $NEW_VERSION'"
echo "  git tag v$NEW_VERSION"
echo "  git push origin main --tags"
