#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::GenericImageView;
use notify::{Event as NotifyEvent, RecursiveMode, Result as NotifyResult, Watcher};
use notify_rust::Notification;
use serde_json::Value;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::{fs, thread};
use tao::event_loop::EventLoop;
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

#[derive(Debug)]
enum WallmeterError {
    CannotReadWallpaperConfig,
    FailedToReadId,
    CannotReadBackupProfile,
    CannotParseWallpaperConfig,
}

fn send_notification(body: &str) {
    println!("{}", body);
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
        Err(err) => {
            println!("{err}");
            return Err(WallmeterError::CannotParseWallpaperConfig);
        }
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

fn load_rainmeter_profile(profile: &str) {
    let exe_path = "C:/Program Files/Rainmeter/Rainmeter.exe";
    let args = ["!LoadLayout", profile];
    match Command::new(exe_path).args(&args).output() {
        Ok(_) => {}
        Err(_) => send_notification(&format!("Failed to apply layout")),
    };
}

fn load_wallmeter_profile(file_content: Vec<u8>) {
    let username = whoami::username();
    let path =
        format!("C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/wallmeter/Rainmeter.ini");
    let layout_file = Path::new(&path);
    let mut file = match File::create(layout_file) {
        Ok(res) => res,
        Err(_) => {
            send_notification(&format!("Failed to open file {}", path));
            return;
        }
    };
    match file.write_all(&file_content) {
        Ok(_) => {}
        Err(_) => {
            send_notification(&format!("Failed to write file {}", path));
            return;
        }
    }

    load_rainmeter_profile("wallmeter");
}

fn save_current() -> Result<(), WallmeterError> {
    let username = whoami::username();
    let wallmeter_path =
        format!("C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/wallmeter/Rainmeter.ini");
    let wallmeter = Path::new(&wallmeter_path);

    if !wallmeter.exists() {
        let path_str =
            format!("C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/@Backup/Rainmeter.ini");
        let path = Path::new(&path_str);
        let mut file = match File::open(path) {
            Ok(res) => res,
            Err(err) => {
                println!("{err:?}");
                return Err(WallmeterError::CannotReadBackupProfile);
            }
        };
        let mut file_content: Vec<u8> = Vec::new();
        match file.read_to_end(&mut file_content) {
            Ok(res) => res,
            Err(err) => {
                println!("{err:?}");
                return Err(WallmeterError::CannotReadBackupProfile);
            }
        };
        create_dir_all(format!(
            "C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/wallmeter/"
        ))
        .unwrap();
        load_wallmeter_profile(file_content);
    } else {
        load_rainmeter_profile("wallmeter");
    }

    let id = get_current_wallpaper_id()?;
    load_rainmeter_profile("@Backup");

    let backup_path =
        format!("C:/Users/{username}/AppData/Roaming/Rainmeter/Layouts/@Backup/Rainmeter.ini");
    let path = Path::new(&backup_path);
    let mut file = match File::open(path) {
        Ok(res) => res,
        Err(err) => {
            println!("{err:?}");
            return Err(WallmeterError::CannotReadBackupProfile);
        }
    };
    let mut file_content: Vec<u8> = Vec::new();
    match file.read_to_end(&mut file_content) {
        Ok(res) => res,
        Err(err) => {
            println!("{err:?}");
            return Err(WallmeterError::CannotReadBackupProfile);
        }
    };

    let layout_path_str = format!("C:/Users/{username}/.wallmeter/profiles/{id}/layout.ini");
    let layout_path = Path::new(&layout_path_str);

    if let Some(dir) = layout_path_str.rsplit('/').next() {
        let dir_path = layout_path_str.strip_suffix(dir).unwrap();
        create_dir_all(dir_path).unwrap();
    }

    let mut file = match File::create(layout_path) {
        Ok(res) => res,
        Err(_) => {
            send_notification(&format!("Failed to open file {:?}", layout_path));
            return Ok(());
        }
    };
    match file.write_all(&file_content) {
        Ok(_) => {}
        Err(_) => send_notification(&format!("Failed to write file {:?}", layout_path)),
    }
    Ok(())
}

enum UserEvent {
    MenuEvent(tray_icon::menu::MenuEvent),
}

fn run_tray_event_loop(event_loop: EventLoop<UserEvent>) {
    let icon_bytes = include_bytes!("../icon.png");
    let icon = image::load_from_memory(icon_bytes).expect("Failed to load image");

    let (width, height) = icon.dimensions();
    let rgba = icon.to_rgba8();
    let icon = Icon::from_rgba(rgba.to_vec(), width, height).unwrap();

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let tray_menu = Menu::new();
    let quit_i = MenuItem::new("Quit", true, None);
    let create_i = MenuItem::new("Save current", true, None);
    tray_menu.append_items(&[&create_i, &quit_i]).unwrap();

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Wallmeter")
        .with_icon(icon)
        .build()
        .unwrap();

    let menu_channel = MenuEvent::receiver();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                if event.id == create_i.id() {
                    match save_current() {
                        Ok(_) => send_notification("Saved current setup"),
                        Err(err) => send_notification(&format!("Error: {:?}", err)),
                    }
                }
                if event.id == quit_i.id() {
                    *control_flow = ControlFlow::Exit;
                }
            }

            _ => {}
        }
    });
}

fn main() -> Result<(), ()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let event_loop_thread = thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<NotifyResult<NotifyEvent>>();
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
                            println!("{err:?}");
                            continue;
                        }
                    };
                    if last_wallpaper != wallpaper_id {
                        let profile_dir = format!(
                            "C:/Users/{username}/.wallmeter/profiles/{wallpaper_id}/layout.ini"
                        );
                        let profile_path = Path::new(&profile_dir);
                        let mut file = match File::open(profile_path) {
                            Ok(res) => res,
                            Err(err) => {
                                println!("{err:?}");
                                println!("Assuming no profile for {wallpaper_id}");
                                continue;
                            }
                        };
                        let mut file_content: Vec<u8> = Vec::new();
                        file.read_to_end(&mut file_content).unwrap();
                        load_wallmeter_profile(file_content);
                    }
                    last_wallpaper = wallpaper_id.clone();
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });

    run_tray_event_loop(event_loop);
    event_loop_thread.join().unwrap();
    Ok(())
}
