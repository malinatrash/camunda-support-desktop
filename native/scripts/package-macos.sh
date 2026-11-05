#!/usr/bin/env bash
# Собирает Camunda Support.app и DMG с ним в target/release/bundle.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo build --release
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
bundle="target/release/bundle"
app="$bundle/macos/Camunda Support.app"
rm -rf "$bundle"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$bundle/dmg"
cp target/release/camunda-support "$app/Contents/MacOS/camunda-support"
cp assets/icons/icon.icns "$app/Contents/Resources/AppIcon.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Camunda Support</string>
  <key>CFBundleDisplayName</key><string>Camunda Support</string>
  <key>CFBundleIdentifier</key><string>com.malinatrash.camundasupport</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleExecutable</key><string>camunda-support</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key><string>com.malinatrash.camundasupport</string>
      <key>CFBundleURLSchemes</key><array><string>camunda-support</string></array>
    </dict>
  </array>
</dict>
</plist>
PLIST
codesign --force --deep --options runtime --sign - "$app"
codesign --verify --deep --strict "$app"

staging="$(mktemp -d)"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
hdiutil create -volname "Camunda Support" -srcfolder "$staging" -ov -format UDZO "$bundle/dmg/Camunda Support_${version}_aarch64.dmg" >/dev/null
rm -rf "$staging"
echo "$bundle/dmg/Camunda Support_${version}_aarch64.dmg"
