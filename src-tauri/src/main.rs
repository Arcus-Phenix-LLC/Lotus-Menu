#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod toolbox;

use std::{path::PathBuf, process::Command, sync::Mutex};
use toolbox::{Menu, ToolConfig};

struct State {
    root: PathBuf,
    snapshot: Mutex<Menu>,
}

fn root_directory() -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Directory")
    } else {
        std::env::current_exe()
            .expect("Cannot locate Lotus Menu")
            .parent()
            .unwrap()
            .join("Directory")
    }
}

#[tauri::command]
fn scan_directory(state: tauri::State<State>) -> Result<Menu, String> {
    let mut cached = state.snapshot.lock().map_err(|e| e.to_string())?;
    let menu = toolbox::scan(&state.root)?;
    *cached = menu.clone();
    Ok(menu)
}

#[tauri::command]
fn save_choice(
    folder: String,
    launch: String,
    name: String,
    state: tauri::State<State>,
) -> Result<(), String> {
    let cached = state.snapshot.lock().map_err(|e| e.to_string())?;
    if !cached
        .choices
        .iter()
        .any(|c| c.folder == folder && c.candidates.contains(&launch))
    {
        return Err("Refresh the menu before choosing this executable.".into());
    }
    toolbox::save_choice(
        &state.root,
        &folder,
        ToolConfig {
            name: Some(name),
            launch,
        },
    )
}

#[tauri::command]
fn launch_tool(id: String, state: tauri::State<State>) -> Result<(), String> {
    let cached = state.snapshot.lock().map_err(|e| e.to_string())?;
    if !toolbox::contains_tool(&cached.items, &id) {
        return Err("This tool is not in the current menu. Refresh first.".into());
    }
    let executable = toolbox::contained_file(&state.root, &id)?;
    if !toolbox::is_executable(&executable) {
        return Err("Only .exe tools are supported in this version.".into());
    }
    let mut child = Command::new(&executable)
        .current_dir(executable.parent().unwrap())
        .spawn()
        .map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[tauri::command]
fn open_folder(state: tauri::State<State>) -> Result<(), String> {
    std::fs::create_dir_all(&state.root).map_err(|e| e.to_string())?;
    let folder = state.root.canonicalize().map_err(|e| e.to_string())?;
    let mut child = Command::new("explorer.exe")
        .arg(folder)
        .spawn()
        .map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(State {
            root: root_directory(),
            snapshot: Mutex::new(Menu::default()),
        })
        .invoke_handler(tauri::generate_handler![
            scan_directory,
            save_choice,
            launch_tool,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("Could not start Lotus Menu");
}
