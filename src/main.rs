#![windows_subsystem = "windows"]
use notify::{Event, RecursiveMode, Result, Watcher};
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::fs;
use std::process::Command;
use std::sync::mpsc;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new("C:/Program Files (x86)/Steam/steamapps/common/wallpaper_engine/config.json"), RecursiveMode::Recursive)?;

    let mut last_wallpaper = String::from("");
    for res in rx {
        match res {
            Ok(_) => {
                let file_content = fs::read_to_string(Path::new("C:/Program Files (x86)/Steam/steamapps/common/wallpaper_engine/config.json"))?;
                let json: Value = serde_json::from_str(&file_content).unwrap();
                if let Some(obj) = json.as_object() {
                    let wallpaper_path = obj
                        .get("Gavi").unwrap().as_object().unwrap()
                        .get("general").unwrap().as_object().unwrap()
                        .get("wallpaperconfig").unwrap().as_object().unwrap()
                        .get("selectedwallpapers").unwrap().as_object().unwrap()
                        .get("Monitor0").unwrap().as_object().unwrap()
                        .get("file").unwrap().as_str().unwrap();

                    if last_wallpaper != wallpaper_path {
                        let parts: Vec<&str> = wallpaper_path.split("431960/").collect();
                        let id_part = parts.get(1);
                        if id_part.is_none() {
                            continue;
                        }
                        let id_parts: Vec<&str> = id_part.unwrap().split("/").collect();
                        let id = id_parts.get(0);
                        if id.is_none() {
                            continue;
                        }

                        let username = whoami::username();
                        let profiles_dir = format!("C:/Users/{username}/.wallmeter/profiles");
                        let profiles_path = Path::new(&profiles_dir);
                        let subpath: PathBuf = profiles_path.join(id.unwrap()).join("layout.ini");
                        let file_content = fs::read_to_string(subpath);
                        if file_content.is_err() {
                            continue;
                        }

                        let path = format!("C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/wallmeter/Rainmeter.ini");
                        let layout_file = Path::new(&path);
                        let mut file = File::create(layout_file).unwrap();
                        file.write_all(file_content.unwrap().as_bytes()).unwrap();

                        let exe_path = "C:/Program Files/Rainmeter/Rainmeter.exe";
                        let args = ["!LoadLayout", "wallmeter"];
                        Command::new(exe_path)
                            .args(&args)
                            .output()
                            .expect("Failed to execute process");
                    }
                    last_wallpaper = wallpaper_path.to_string();
                }
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}