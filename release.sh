#!/usr/bin/env bash
set -euo pipefail

cargo build --release

BIN="target/release/vibetone"
echo "Built: $BIN ($(du -h "$BIN" | cut -f1))"

# Detect OS and install appropriately
case "$(uname -s)" in
  Darwin)
    APP="target/release/Vibetone.app"
    rm -rf "$APP"
    mkdir -p "$APP/Contents/MacOS"
    mkdir -p "$APP/Contents/Resources"

    cp "$BIN" "$APP/Contents/MacOS/vibetone"
    cp assets/icon.icns "$APP/Contents/Resources/AppIcon.icns"

    cat > "$APP/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Vibetone</string>
    <key>CFBundleDisplayName</key>
    <string>Vibetone</string>
    <key>CFBundleIdentifier</key>
    <string>com.gantistorm.vibetone</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>vibetone</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>Vibetone needs microphone access for sidetone monitoring.</string>
</dict>
</plist>
PLIST

    cp "$APP" /Applications/Vibetone.app 2>/dev/null && \
      echo "Installed to /Applications/Vibetone.app" || \
      (cp -r "$APP" /Applications/Vibetone.app && echo "Installed to /Applications/Vibetone.app")

    echo "Open from Spotlight or:  open /Applications/Vibetone.app"
    ;;

  Linux)
    DEST="$HOME/.local/bin"
    mkdir -p "$DEST"
    cp "$BIN" "$DEST/vibetone"

    # Desktop entry
    mkdir -p "$HOME/.local/share/applications"
    cat > "$HOME/.local/share/applications/vibetone.desktop" << DESKTOP
[Desktop Entry]
Name=Vibetone
Comment=Real-time sidetone for vibe coders
Exec=$DEST/vibetone
Terminal=false
Type=Application
Categories=Audio;
DESKTOP

    echo "Installed to $DEST/vibetone"
    echo "Run from app launcher or:  vibetone"
    ;;

  *)
    echo "Unknown OS. Binary is at: $BIN"
    ;;
esac
