#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$DIR/dist"
STAGING="$DIST/deb-staging"

echo "=================================================="
echo "    📦 Building SIDM Debian / Ubuntu Package (.deb) "
echo "=================================================="

rm -rf "$STAGING"
mkdir -p "$DIST" "$STAGING/DEBIAN"
mkdir -p "$STAGING/opt/sidm/bin" "$STAGING/opt/sidm/extension"
mkdir -p "$STAGING/usr/bin"
mkdir -p "$STAGING/usr/share/applications"
mkdir -p "$STAGING/usr/share/icons/hicolor/128x128/apps"
mkdir -p "$STAGING/usr/share/icons/hicolor/48x48/apps"

# 1. Binaries
cp "$DIR/sidm-core/target/release/sidm-core" "$STAGING/opt/sidm/sidm-core"
chmod 755 "$STAGING/opt/sidm/sidm-core"

if [ -f "$DIR/bin/yt-dlp" ]; then
    cp "$DIR/bin/yt-dlp" "$STAGING/opt/sidm/bin/yt-dlp"
    chmod 755 "$STAGING/opt/sidm/bin/yt-dlp"
fi

# 2. Extension
cp -r "$DIR/sidm-extension/"* "$STAGING/opt/sidm/extension/"

# 3. CLI Wrapper
cat << 'EOF' > "$STAGING/usr/bin/sidm"
#!/usr/bin/env bash
set -e

if [ "$1" == "stop" ]; then
    pkill -f "sidm-core" || true
    echo "⚡ SIDM Core stopped."
    exit 0
fi

if [ "$1" == "restart" ]; then
    pkill -f "sidm-core" || true
    sleep 0.5
fi

# Launch daemon in background if not already running
if ! pgrep -f "sidm-core server" >/dev/null 2>&1; then
    /opt/sidm/sidm-core server >/dev/null 2>&1 &
    sleep 0.5
fi

# Open UI
if command -v google-chrome >/dev/null 2>&1; then
    google-chrome --app=http://127.0.0.1:45678 >/dev/null 2>&1 &
elif command -v chromium-browser >/dev/null 2>&1; then
    chromium-browser --app=http://127.0.0.1:45678 >/dev/null 2>&1 &
elif command -v chromium >/dev/null 2>&1; then
    chromium --app=http://127.0.0.1:45678 >/dev/null 2>&1 &
elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open http://127.0.0.1:45678 >/dev/null 2>&1 &
fi
EOF
chmod 755 "$STAGING/usr/bin/sidm"

# 4. Desktop Entry & Icons
cat << 'EOF' > "$STAGING/usr/share/applications/sidm.desktop"
[Desktop Entry]
Name=SIDM
GenericName=Internet Download Manager
Comment=Smart Internet Download Manager (16x Multi-Stream Accelerator)
Exec=/usr/bin/sidm
Icon=sidm
Terminal=false
Type=Application
Categories=Network;FileTransfer;
StartupNotify=true
EOF
chmod 644 "$STAGING/usr/share/applications/sidm.desktop"

cp "$DIR/sidm-extension/icons/icon128.png" "$STAGING/usr/share/icons/hicolor/128x128/apps/sidm.png"
cp "$DIR/sidm-extension/icons/icon48.png" "$STAGING/usr/share/icons/hicolor/48x48/apps/sidm.png"

# 5. Chrome / Edge Extension Auto-Registration on Linux
mkdir -p "$STAGING/usr/share/google-chrome/extensions" "$STAGING/usr/share/chromium/extensions"
cat << 'EOF' > "$STAGING/usr/share/google-chrome/extensions/gadhilaplipaklonafepmbegnhoafice.json"
{
  "external_crx": "/opt/sidm/extension",
  "external_version": "1.0.0"
}
EOF
cp "$STAGING/usr/share/google-chrome/extensions/gadhilaplipaklonafepmbegnhoafice.json" "$STAGING/usr/share/chromium/extensions/"

# 6. Debian Control File
cat << 'EOF' > "$STAGING/DEBIAN/control"
Package: sidm
Version: 1.1.0
Section: net
Priority: optional
Architecture: amd64
Maintainer: SFIDM Team <support@sidm.app>
Depends: libc6 (>= 2.31), ca-certificates
Recommends: ffmpeg
Homepage: https://github.com/kawsarapp/SFIDM
Description: Smart Internet Download Manager (SIDM)
 Next-Gen High-Speed Download Manager for Linux & Windows.
 Accelerates downloads by up to 16x with multi-thread segmenting,
 floating browser video sniffer, and native Adobe Premiere Pro ready MP4s.
EOF

# 7. Post-install script
cat << 'EOF' > "$STAGING/DEBIAN/postinst"
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 "$STAGING/DEBIAN/postinst"
chmod 755 "$STAGING/DEBIAN"

# Build DEB
dpkg-deb --build --root-owner-group "$STAGING" "$DIST/sidm_1.1.0_amd64.deb"
cp "$DIST/sidm_1.1.0_amd64.deb" "$DIST/sidm_1.0.0_amd64.deb"
rm -rf "$STAGING"

echo "✅ Linux .deb package created successfully at:"
echo "   $DIST/sidm_1.0.0_amd64.deb"
