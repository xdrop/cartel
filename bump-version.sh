#!/usr/bin/env bash
set -euxo pipefail

if [ $# -ne 1 ]; then
    echo "Error: This script requires exactly one argument (the new version)"
    exit 1
fi

NEW_VERSION=$1
OLD_VERSION=$(cat VERSION)

sed_cmd() {
    if command -v gsed &> /dev/null; then
        # GNU sed (gsed) on macOS
        gsed "$@"
    else
        sed "$@"
    fi
}

echo "Updating VERSION"
echo $NEW_VERSION > VERSION

echo "Updating CHANGELOG.md..."
sed_cmd -i'' -e "s;## \[Unreleased\];## [Unreleased]\n\n## [$NEW_VERSION] - $(date "+%Y-%m-%d");g" CHANGELOG.md

echo "Updating Cargo.toml..."
sed_cmd -i'' -e "s/^version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/g" Cargo.toml

echo "Trigger lockfile update..."
touch src/client/cli.rs && cargo check

echo "Committing.."
git add 'Cargo.toml' 'Cargo.lock' 'CHANGELOG.md' 'VERSION'
git commit -m "Bump to $NEW_VERSION"

echo "Performing git tag..."
git tag -f $NEW_VERSION

