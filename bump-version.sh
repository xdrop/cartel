NEW_VERSION=$1
OLD_VERSION=$(cat VERSION)
echo "Updating VERSION"
echo $NEW_VERSION > VERSION
echo "Updating CHANGELOG.md..."
sed -i "s;## \[Unreleased\];## [Unreleased]\n\n## [$NEW_VERSION-beta] - $(date "+%Y-%m-%d");g" CHANGELOG.md
echo "Updating Cargo.toml..."
sed -i "s/^version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/g" Cargo.toml
echo "Trigger lockfile update..."
touch src/client/cli.rs && cargo check
echo "Committing.."
git add 'Cargo.toml' 'Cargo.lock' 'CHANGELOG.md' 'VERSION'
git commit -m "Bump to $NEW_VERSION (beta)"
echo "Performing git tag..."
git tag $NEW_VERSION-beta

