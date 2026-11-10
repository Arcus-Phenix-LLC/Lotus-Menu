# Lotus Menu

A portable Windows toolbox with a Windows Vista/7 inspired default theme. Drop tools into folders and Lotus builds a cascading menu on startup. Use **Refresh Menu** after changes; there is no background folder watcher.

## Organizing tools

```text
Lotus Menu.exe
Directory/
  Network/
    Scanner/
      Scanner.exe
      Updater.exe
      lotus.json       # saved by Lotus after you choose Scanner.exe
  Storage/
    DiskInspector/
      DiskInspector.exe
```

Folders without executables become categories. A folder with one `.exe` becomes one tool, named after the folder. With multiple executables, Lotus asks you to choose and saves a relative launch path in `lotus.json`. Supporting subfolders inside a tool are not scanned. Put each application in its own folder; do not mix category subfolders and application executables in the same folder. Standalone executables directly in `Directory` are supported too.

Saved choices travel with the drive. Release builds always find `Directory` beside the launcher executable, regardless of drive letter or current working directory. Development builds use this repository’s `Directory`. Applications launch with their executable’s parent folder as the working directory. A missing or invalid saved executable triggers a new choice; folders with no remaining executable show a warning. Selection requires a writable drive.

This first version supports Windows `.exe` tools. Script files, shortcuts, launch arguments, per-tool elevation, and additional themes are future work. To run tools requiring administrator access, run Lotus as administrator. Legacy Vista/7 is a visual reference, not a supported runtime target. Linked folders are skipped. Folder depth is limited to 32 levels.

## Build on Windows 10/11

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **Desktop development with C++**, and Microsoft Edge WebView2 Runtime. See [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows).
2. Install Rust with [rustup](https://rustup.rs/). This project pins Rust 1.93.0; rustup installs it when you run Cargo from this directory.
3. Install the matching major version of the CLI:
   ```powershell
   cargo install tauri-cli --version "^2" --locked
   ```
4. From this repository’s root:
   ```powershell
   cargo tauri dev
   ```

No Node.js, frontend package installation, CDN, or network connection is needed at runtime. Crates and the compiler need downloading on the first build unless already cached.

For a portable release executable:

```powershell
cargo tauri build --no-bundle
```

Copy `src-tauri/target/release/lotus-menu.exe` to your flash drive as `Lotus Menu.exe`, with a `Directory` folder beside it. Do not copy the source `src` folder; the UI is embedded. The destination Windows machine needs WebView2 installed. No installer is generated in this version.

## Checks

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
node --check src/app.js
```

Rust tests cover category discovery, ambiguous executables, persisted choices, relocation, missing files, invalid configuration, traversal rejection, and hiding support executables. They use temporary dummy files and never launch tools.

## Design and future themes

`src/app.js` owns interaction; `src/themes/aero.css` owns the default theme. `data-theme="aero"` identifies the current theme. Future stylesheets can reuse the markup and semantic menu behavior. There is no theme picker until additional themes exist. Mouse, Tab, Enter, and arrow keys navigate the menus. Deep menus scroll horizontally within the window.

The Rust/Tauri 2 backend exposes only scan, saved-choice, launch, and browse commands. The Python prototype has been removed; Lotus Menu uses Rust/Tauri. The original Lotus artwork, `Lotus_boot.png`, is preserved in the repository. The working source originated from `Arcus-Phenix-LLC/Lotus-Menu` main at `3a11182cf3b08e4e5ac6092046fcab83b72f83e0`.
