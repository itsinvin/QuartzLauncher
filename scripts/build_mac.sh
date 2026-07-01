#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Missing version argument"
    exit 1
fi

version=${1#v}
export PANDORA_RELEASE_VERSION=$version

cargo build --release --frozen --target aarch64-apple-darwin
cargo build --release --frozen --target x86_64-apple-darwin

strip target/aarch64-apple-darwin/release/quartz_launcher
strip target/x86_64-apple-darwin/release/quartz_launcher

mkdir -p dist

lipo -create -output dist/QuartzLauncher-macOS-Universal target/x86_64-apple-darwin/release/quartz_launcher target/aarch64-apple-darwin/release/quartz_launcher

cargo install cargo-packager
env -u CARGO_PACKAGER_SIGN_PRIVATE_KEY cargo packager --config '{'\
'  "name": "pandora-launcher",'\
'  "outDir": "./dist",'\
'  "formats": ["dmg", "app"],'\
'  "productName": "QuartzLauncher",'\
'  "version": "'"$version"'",'\
'  "identifier": "com.moulberry.pandoralauncher",'\
'  "resources": [],'\
'  "authors": ["Moulberry"],'\
'  "binaries": [{ "path": "QuartzLauncher-macOS-Universal", "main": true }],'\
'  "icons": ["package/mac.icns"],'\
'  "macos": {"entitlements": "package/mac/entitlements.plist", "infoPlistPath": "package/mac/Info.plist"}'\
'}'

mv -f dist/QuartzLauncher-macOS-Universal dist/QuartzLauncher-macOS-Universal-Portable
mv -f dist/QuartzLauncher*.dmg dist/QuartzLauncher.dmg
tar -czf dist/QuartzLauncher.app.tar.gz dist/QuartzLauncher.app
rm -r dist/QuartzLauncher.app

if [[ -n "$CARGO_PACKAGER_SIGN_PRIVATE_KEY" ]]; then
    cargo packager signer sign dist/QuartzLauncher-macOS-Universal-Portable
    cargo packager signer sign dist/QuartzLauncher.dmg
    cargo packager signer sign dist/QuartzLauncher.app.tar.gz

    echo "{
    \"version\": \"$version\",
    \"downloads\": {
        \"universal\": {
            \"executable\": {
                \"download\": \"https://github.com/itsinvin/QuartzLauncher/releases/download/v$version/QuartzLauncher-macOS-Universal-Portable\",
                \"size\": $(wc -c < dist/QuartzLauncher-macOS-Universal-Portable),
                \"sha1\": \"$(sha1sum dist/QuartzLauncher-macOS-Universal-Portable | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/QuartzLauncher-macOS-Universal-Portable.sig)\"
            },
            \"app\": {
                \"download\": \"https://github.com/itsinvin/QuartzLauncher/releases/download/v$version/QuartzLauncher.app.tar.gz\",
                \"size\": $(wc -c < dist/QuartzLauncher.app.tar.gz),
                \"sha1\": \"$(sha1sum dist/QuartzLauncher.app.tar.gz | cut -d ' ' -f 1)\",
                \"sig\": \"$(cat dist/QuartzLauncher.app.tar.gz.sig)\"
            }
        }
    }
}" > dist/update_manifest_macos.json

    rm dist/*.sig
fi
