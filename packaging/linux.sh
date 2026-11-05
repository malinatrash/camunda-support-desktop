#!/usr/bin/env bash
# Собирает deb-пакет camunda-support в target/release/bundle/deb.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo build --release --package camunda-support
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
root="target/release/bundle/deb/camunda-support_${version}_amd64"
rm -rf "$root"
mkdir -p "$root/DEBIAN" "$root/usr/bin" "$root/usr/share/applications" "$root/usr/share/icons/hicolor/128x128/apps" "$root/usr/share/icons/hicolor/512x512/apps"
install -m 755 target/release/camunda-support "$root/usr/bin/camunda-support"
install -m 644 assets/icons/128x128.png "$root/usr/share/icons/hicolor/128x128/apps/camunda-support.png"
install -m 644 assets/icons/icon.png "$root/usr/share/icons/hicolor/512x512/apps/camunda-support.png"
cat > "$root/usr/share/applications/camunda-support.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Camunda Support
Comment=Поддержка процессов Camunda
Exec=camunda-support %u
Icon=camunda-support
Terminal=false
Categories=Development;
MimeType=x-scheme-handler/camunda-support;
StartupWMClass=camunda-support
DESKTOP
cat > "$root/DEBIAN/control" <<CONTROL
Package: camunda-support
Version: $version
Section: devel
Priority: optional
Architecture: amd64
Maintainer: malinatrash
Depends: libc6, libfontconfig1, libfreetype6, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libx11-xcb1, libxcb1, libvulkan1, libssl3 | libssl3t64
Description: Camunda Support
 Нативная консоль поддержки процессов Camunda.
CONTROL
cat > "$root/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
command -v xdg-mime >/dev/null && xdg-mime default camunda-support.desktop x-scheme-handler/camunda-support || true
POSTINST
chmod 755 "$root/DEBIAN/postinst"
dpkg-deb --build --root-owner-group "$root" "${root}.deb" >/dev/null
rm -rf "$root"
echo "${root}.deb"
