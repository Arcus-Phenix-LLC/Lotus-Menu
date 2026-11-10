use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path},
};

#[derive(Clone, Default, Serialize)]
pub struct Menu {
    pub items: Vec<Item>,
    pub choices: Vec<Choice>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct Item {
    pub name: String,
    pub id: String,
    pub children: Option<Vec<Item>>,
}

#[derive(Clone, Serialize)]
pub struct Choice {
    pub folder: String,
    pub name: String,
    pub candidates: Vec<String>,
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: Option<String>,
    pub launch: String,
}

pub fn is_executable(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

fn contained(root: &Path, relative: &str) -> Result<std::path::PathBuf, String> {
    let rel = Path::new(relative);
    if rel.as_os_str().is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err("Expected a relative path inside the tools folder.".into());
    }
    let base = root.canonicalize().map_err(|e| e.to_string())?;
    let mut current = base.clone();
    for part in rel.components() {
        current.push(part);
        if fs::symlink_metadata(&current)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Linked paths are not supported.".into());
        }
    }
    let path = current.canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(&base) {
        return Err("Path leaves the tools folder.".into());
    }
    Ok(path)
}

pub fn contained_file(root: &Path, rel: &str) -> Result<std::path::PathBuf, String> {
    let path = contained(root, rel)?;
    if !path.is_file() {
        return Err("The launch file no longer exists. Refresh the menu.".into());
    }
    Ok(path)
}

pub fn save_choice(root: &Path, folder: &str, config: ToolConfig) -> Result<(), String> {
    let dir = contained(root, folder)?;
    if !dir.is_dir() {
        return Err("Tool folder no longer exists.".into());
    }
    let target = contained_file(&dir, &config.launch)?;
    if !is_executable(&target) {
        return Err("Choose an .exe file.".into());
    }
    let destination = dir.join("lotus.json");
    if destination.exists()
        && fs::symlink_metadata(&destination)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("Cannot overwrite linked configuration.".into());
    }
    let json = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(destination, json)
        .map_err(|e| format!("Could not save the choice (is the drive writable?): {e}"))
}

pub fn contains_tool(items: &[Item], id: &str) -> bool {
    items.iter().any(|item| match &item.children {
        Some(children) => contains_tool(children, id),
        None => item.id == id,
    })
}

pub fn scan(root: &Path) -> Result<Menu, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut menu = Menu::default();
    menu.items = scan_folder(&root, &root, true, 0, &mut menu)?;
    Ok(menu)
}

fn scan_folder(
    root: &Path,
    dir: &Path,
    is_root: bool,
    depth: usize,
    menu: &mut Menu,
) -> Result<Vec<Item>, String> {
    if depth > 32 {
        return Err("Folder nesting exceeds 32 levels.".into());
    }
    let mut entries = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    let mut executables = Vec::new();
    let mut folders = Vec::new();
    for entry in entries {
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            folders.push(entry.path());
        } else if kind.is_file() && is_executable(&entry.path()) {
            executables.push(entry.path());
        }
    }
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let config_path = dir.join("lotus.json");
    let mut reason = "Choose the executable that starts this tool.".to_string();
    if !is_root && config_path.exists() {
        let configured = (|| -> Result<Item, String> {
            if fs::symlink_metadata(&config_path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Linked configuration is not supported.".into());
            }
            let config: ToolConfig =
                serde_json::from_slice(&fs::read(&config_path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let executable = contained_file(dir, &config.launch)?;
            if !is_executable(&executable) {
                return Err("Saved launch file is not an .exe.".into());
            }
            Ok(Item {
                name: config
                    .name
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or(name.clone()),
                id: relative(root, &executable),
                children: None,
            })
        })();
        match configured {
            Ok(tool) => return Ok(vec![tool]),
            Err(error) => {
                reason = format!("Saved choice needs updating: {error}");
            }
        }
    }
    if !is_root && (!executables.is_empty() || config_path.exists()) {
        if executables.len() == 1 && !config_path.exists() {
            return Ok(vec![Item {
                name,
                id: relative(root, &executables[0]),
                children: None,
            }]);
        }
        if executables.is_empty() {
            menu.warnings.push(format!(
                "{}: {reason} No executable found directly in this tool folder.",
                relative(root, dir)
            ));
        } else {
            menu.choices.push(Choice {
                folder: relative(root, dir),
                name,
                candidates: executables.iter().map(|p| relative(dir, p)).collect(),
                reason,
            });
        }
        return Ok(Vec::new());
    }
    let mut items: Vec<Item> = executables
        .iter()
        .map(|p| Item {
            name: p.file_stem().unwrap().to_string_lossy().into_owned(),
            id: relative(root, p),
            children: None,
        })
        .collect();
    for folder in folders {
        match scan_folder(root, &folder, false, depth + 1, menu) {
            Ok(children) if !children.is_empty() => {
                // A tool folder returns its tool; a category wraps its children.
                let is_tool = children.len() == 1
                    && children[0].children.is_none()
                    && Path::new(&children[0].id).parent() == folder.strip_prefix(root).ok();
                if is_tool || folder.join("lotus.json").exists() {
                    items.extend(children);
                } else {
                    items.push(Item {
                        name: folder.file_name().unwrap().to_string_lossy().into_owned(),
                        id: relative(root, &folder),
                        children: Some(children),
                    });
                }
            }
            Ok(_) => (),
            Err(error) => menu
                .warnings
                .push(format!("{}: {error}", relative(root, &folder))),
        }
    }
    items.sort_by_key(|i| (i.children.is_none(), i.name.to_lowercase()));
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!("lotus-test-{}-{unique}", std::process::id())))
        }
        fn file(&self, name: &str) {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "fixture").unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn categories_and_single_tools() {
        let f = Fixture::new();
        f.file("Network/Scanner/scan.EXE");
        f.file("Loose.exe");
        let m = scan(&f.0).unwrap();
        assert!(m.choices.is_empty());
        assert_eq!(m.items[0].name, "Network");
        assert_eq!(m.items[0].children.as_ref().unwrap()[0].name, "Scanner");
        assert!(contains_tool(&m.items, "Network/Scanner/scan.EXE"));
    }
    #[test]
    fn saves_choice_and_survives_drive_move() {
        let f = Fixture::new();
        f.file("Network/Scanner/main.exe");
        f.file("Network/Scanner/helper.exe");
        assert_eq!(scan(&f.0).unwrap().choices.len(), 1);
        save_choice(
            &f.0,
            "Network/Scanner",
            ToolConfig {
                name: Some("Scanner".into()),
                launch: "main.exe".into(),
            },
        )
        .unwrap();
        let moved = Fixture::new();
        fs::rename(&f.0, &moved.0).unwrap();
        let m = scan(&moved.0).unwrap();
        assert!(m.choices.is_empty());
        assert!(contains_tool(&m.items, "Network/Scanner/main.exe"));
        fs::remove_file(moved.0.join("Network/Scanner/main.exe")).unwrap();
        assert_eq!(scan(&moved.0).unwrap().choices.len(), 1);
    }
    #[test]
    fn refuses_escape_and_does_not_descend_into_tool_support_folders() {
        let f = Fixture::new();
        f.file("Tool/main.exe");
        f.file("Tool/support/helper.exe");
        let m = scan(&f.0).unwrap();
        assert!(!contains_tool(&m.items, "Tool/support/helper.exe"));
        assert!(contained_file(&f.0, "../outside.exe").is_err());
        assert!(contained_file(&f.0, "C:/outside.exe").is_err());
    }
    #[test]
    fn malformed_config_is_reported() {
        let f = Fixture::new();
        f.file("Tool/main.exe");
        f.file("Tool/lotus.json");
        let m = scan(&f.0).unwrap();
        assert_eq!(m.choices.len(), 1);
        assert!(m.choices[0].reason.contains("Saved choice"));
    }
}
