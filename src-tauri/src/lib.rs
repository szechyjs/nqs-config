use std::time::Duration;
use tauri::Emitter;

mod commands;
mod isotp;
mod kwp;
mod nqs;
mod slcan;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_ports,
            commands::read_config,
            commands::write_config,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            {
                let app_menu = tauri::menu::Submenu::with_items(
                    &handle,
                    "NQS Config",
                    true,
                    &[
                        &tauri::menu::PredefinedMenuItem::about(
                            &handle,
                            Some("About NQS Config"),
                            None,
                        )?,
                        &tauri::menu::PredefinedMenuItem::separator(&handle)?,
                        &tauri::menu::PredefinedMenuItem::quit(&handle, None)?,
                    ],
                )?;

                let edit_menu = tauri::menu::Submenu::with_items(
                    &handle,
                    "Edit",
                    true,
                    &[
                        &tauri::menu::PredefinedMenuItem::cut(&handle, None)?,
                        &tauri::menu::PredefinedMenuItem::copy(&handle, None)?,
                        &tauri::menu::PredefinedMenuItem::paste(&handle, None)?,
                        &tauri::menu::PredefinedMenuItem::select_all(&handle, None)?,
                    ],
                )?;

                let menu = tauri::menu::Menu::with_items(&handle, &[&app_menu, &edit_menu])?;

                app.set_menu(menu)?;
            }

            tauri::async_runtime::spawn(async move {
                let mut last: Vec<String> = vec![];
                loop {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    let current = commands::list_ports();
                    if current != last {
                        last = current.clone();
                        let _ = handle.emit("ports-changed", current);
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
