# Icons

- `source.svg`: the drop with no plate. Source for the PNG and `.ico` icons
  (Windows and Linux).
- `source-macos.svg`: the drop on Apple's icon grid. Source for `icon.icns`,
  the macOS fallback and the dev Dock icon.
- `Squirt.icon/`: Icon Composer icon for macOS 26 and later. It follows the
  user's icon style (default, dark, clear, tinted). Building it needs Xcode 26.
  Without Xcode, the app falls back to `icon.icns`.

To regenerate, render each SVG to a 1024 px PNG, then run `npx tauri icon`:

    chrome --headless=new --default-background-color=00000000 \
      --window-size=1024,1024 --screenshot=plain.png icons/source.svg
    npx tauri icon plain.png

For macOS, render `source-macos.svg` the same way, run
`npx tauri icon plated.png -o tmp`, and copy `tmp/icon.icns` to `icons/icon.icns`.
