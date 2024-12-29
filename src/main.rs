#![windows_subsystem = "windows"]
use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
use notify_rust::Notification;
use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

#[derive(Debug)]
enum WallmeterError {
    CannotReadWallpaperConfig,
    FailedToReadId,
}

fn send_notification(body: &str) {
    Notification::new()
        .summary("Wallmeter")
        .body(body)
        .show()
        .unwrap();
}

fn get_current_wallpaper_id() -> Result<String, WallmeterError> {
    let config_path =
        Path::new("C:/Program Files (x86)/Steam/steamapps/common/wallpaper_engine/config.json");
    let file_content = match fs::read_to_string(config_path) {
        Ok(res) => res,
        Err(_) => return Err(WallmeterError::CannotReadWallpaperConfig),
    };
    let json: Value = match serde_json::from_str(&file_content) {
        Ok(res) => res,
        Err(_) => return Err(WallmeterError::CannotReadWallpaperConfig),
    };

    let username = whoami::username();

    let wallpaper_path = json
        .as_object()
        .unwrap()
        .get(&username)
        .unwrap()
        .as_object()
        .unwrap()
        .get("general")
        .unwrap()
        .as_object()
        .unwrap()
        .get("wallpaperconfig")
        .unwrap()
        .as_object()
        .unwrap()
        .get("selectedwallpapers")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Monitor0")
        .unwrap()
        .as_object()
        .unwrap()
        .get("file")
        .unwrap()
        .as_str()
        .unwrap();

    let parts: Vec<&str> = wallpaper_path.split("431960/").collect();
    let id_part = match parts.get(1) {
        Some(res) => res,
        None => return Err(WallmeterError::FailedToReadId),
    };

    let id_parts: Vec<&str> = id_part.split("/").collect();
    let id = match id_parts.get(0) {
        Some(res) => res,
        None => return Err(WallmeterError::FailedToReadId),
    };

    Ok(id.to_string())
}

fn main() -> Result<(), ()> {
    let (tx, rx) = mpsc::channel::<NotifyResult<Event>>();
    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(res) => res,
        Err(_) => {
            send_notification("Failed to watch");
            panic!("Failed to watch")
        }
    };
    match watcher.watch(
        Path::new("C:/Program Files (x86)/Steam/steamapps/common/wallpaper_engine/config.json"),
        RecursiveMode::Recursive,
    ) {
        Ok(_) => {}
        Err(_) => {
            send_notification("Failed to watch");
            panic!("Failed to watch")
        }
    };

    let mut last_wallpaper = String::from("");
    let username = whoami::username();
    for res in rx {
        match res {
            Ok(_) => {
                let wallpaper_id = match get_current_wallpaper_id() {
                    Ok(res) => res,
                    Err(err) => {
                        send_notification(&format!("Error: {:?}", err));
                        continue;
                    }
                };
                if last_wallpaper != wallpaper_id {
                    let profiles_dir = format!("C:/Users/{username}/.wallmeter/profiles");
                    let profiles_path = Path::new(&profiles_dir);
                    let subpath: PathBuf =
                        profiles_path.join(wallpaper_id.clone()).join("layout.ini");
                    let file_content = match fs::read_to_string(subpath) {
                        Ok(res) => res,
                        Err(_) => continue,
                    };

                    let path = format!("C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/wallmeter/Rainmeter.ini");
                    let layout_file = Path::new(&path);
                    let mut file = match File::create(layout_file) {
                        Ok(res) => res,
                        Err(_) => {
                            send_notification(&format!("Failed to open file {}", path));
                            continue;
                        }
                    };
                    match file.write_all(file_content.as_bytes()) {
                        Ok(_) => {}
                        Err(_) => send_notification(&format!("Failed to write file {}", path)),
                    }

                    let exe_path = "C:/Program Files/Rainmeter/Rainmeter.exe";
                    let args = ["!LoadLayout", "wallmeter"];
                    match Command::new(exe_path).args(&args).output() {
                        Ok(_) => {}
                        Err(_) => send_notification(&format!("Failed to apply layout")),
                    };
                }
                last_wallpaper = wallpaper_id.clone();
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
