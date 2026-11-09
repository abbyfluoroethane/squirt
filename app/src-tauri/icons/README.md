# Icons

- `source.svg`: the folded drop, no plate. `npx tauri icon` renders it into
  the PNG and `.ico` icons (Windows and Linux use these plate-free).
- `source-macos.svg`: the drop on Apple's icon grid (824 pt squircle plate in a
  1024 canvas). Rendered into `icon.icns`, the macOS fallback and dev Dock icon.
- `Squirt.icon/`: Icon Composer icon for macOS 26+. Three glass layers (front,
  middle, back folds) on a light plate, charcoal in Dark mode; macOS derives
  Clear and Tinted. It follows the user's icon style setting. The Tauri bundler
  compiles it into `Assets.car` with Xcode 26's `actool`. Without full Xcode,
  the bundler skips it and the app uses `icon.icns`.

To regenerate (renders SVG to PNG with headless Chrome):

    chrome --headless=new --default-background-color=00000000 \
      --window-size=1024,1024 --screenshot=plain.png icons/source.svg
    npx tauri icon plain.png
    # then render source-macos.svg the same way, run `npx tauri icon plated.png -o tmp`
    # and copy tmp/icon.icns over icons/icon.icns
