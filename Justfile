# run in development mode
dev:
    npm run tauri dev

# build release
build:
    npm run build
    cargo build --release --manifest-path src-tauri/Cargo.toml

# build and install the app to /Applications
install:
    npm run build
    npm run tauri build
    @echo "Installing to /Applications..."
    rm -rf /Applications/ezwhisper.app
    cp -r src-tauri/target/release/bundle/macos/ezwhisper.app /Applications/
    @echo "Installed! Launch from /Applications or Spotlight"

# uninstall
uninstall:
    rm -rf /Applications/ezwhisper.app
    rm -rf ~/Library/Application\ Support/com.piotrostr.ezwhisper
    @echo "Uninstalled"

# open permission settings
permissions:
    #!/usr/bin/env bash
    echo "Opening permission settings - add ezwhisper.app to each"
    echo ""
    echo "1. Input Monitoring (for button detection)"
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
    read -p "Press Enter after granting Input Monitoring..."
    echo ""
    echo "2. Accessibility (for keyboard simulation)"
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    read -p "Press Enter after granting Accessibility..."
    echo ""
    echo "3. Microphone (for audio recording)"
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
    read -p "Press Enter after granting Microphone..."
    echo ""
    echo "All permissions configured!"
