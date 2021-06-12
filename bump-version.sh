NEW_VERSION=$1
OLD_VERSION=$(cat VERSION)
echo "Updating VERSION"
echo $NEW_VERSION > VERSION
echo "Updating Cargo.toml..."
sed -i "s/^version = \"$OLD_VERSION\"/version = \"$NEW_VERSION\"/g" Cargo.toml
echo "Trigger lockfile update..."
touch src/client/cli.rs && cargo check

