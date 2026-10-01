# Installing LaserCAD

Download the file for your system from the
[GitHub releases page](https://github.com/grstein/LaserCAD-R14-V2/releases).
To build from source instead, see [`build-local.md`](build-local.md).

| System | File |
|---|---|
| Linux x86-64 | `lasercad-x86_64.AppImage` or `lasercad_<version>_amd64.deb` |
| Windows x86-64 | `lasercad-<version>-windows-x86_64.zip` |
| macOS, Apple Silicon | `lasercad-<version>-macos-aarch64.dmg` |

Intel Macs have no download; build from source.

## The app is unsigned

LaserCAD has no code-signing certificate and is not notarized by Apple. Windows and macOS
therefore warn the first time you open it. The warning only says that the publisher is
unknown; you confirm once and later starts open normally.

## Linux

- **AppImage**: `chmod +x lasercad-x86_64.AppImage`, then run `./lasercad-x86_64.AppImage`.
- **Debian / Ubuntu**: `sudo apt install ./lasercad_<version>_amd64.deb`, then start
  LaserCAD from the application menu or run `lasercad`.

## Windows

1. Unzip `lasercad-<version>-windows-x86_64.zip` into any folder, for example beside LaserGRBL.
   Nothing is installed; the folder holds `lasercad.exe`, the licences and `FIRST-RUN.txt`.
2. Double-click `lasercad.exe`.
3. SmartScreen may show "Windows protected your PC". Click **More info**, then **Run anyway**.

To remove LaserCAD, delete the folder (and, if you like, the files listed below).

## macOS

1. Open `lasercad-<version>-macos-aarch64.dmg` and drag `LaserCAD.app` onto `Applications`.
2. The first time, right-click (or Control-click) `LaserCAD.app` in Applications, choose
   **Open**, then **Open** again in the Gatekeeper dialog. A plain double-click only offers to
   move the app to the Trash.
3. If macOS still refuses ("LaserCAD is damaged and can't be opened"), clear the download
   quarantine flag in Terminal, then open the app normally:

   ```bash
   xattr -dr com.apple.quarantine /Applications/LaserCAD.app
   ```

## Where LaserCAD keeps its files

Settings hold the preferences, the default bed size for new drawings, the AI settings and the recent-files list
(`recent_files`). The autosave file is the crash-recovery copy of the open drawing. Your
drawings are saved wherever you choose.

| System | Settings and recent files | Autosave |
|---|---|---|
| Linux | `~/.config/lasercad/settings.json` | `~/.local/share/lasercad/autosave.json` |
| Windows | `%APPDATA%\lasercad\config\settings.json` | `%LOCALAPPDATA%\lasercad\data\autosave.json` |
| macOS | `~/Library/Application Support/lasercad/settings.json` | `~/Library/Application Support/lasercad/autosave.json` |

On Linux, `$XDG_CONFIG_HOME` and `$XDG_DATA_HOME` replace `~/.config` and `~/.local/share`
when they are set.
