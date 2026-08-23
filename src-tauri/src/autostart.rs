// src-tauri/src/autostart.rs
use tauri::AppHandle;

pub fn setup_autostart(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart_manager = app.autolaunch();
        autostart_manager.enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart_manager = app.autolaunch();
        autostart_manager.enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart_manager = app.autolaunch();
        autostart_manager.enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    }
    
    Ok(())
}

pub fn setup_single_instance(_app: &AppHandle) -> Result<(), String> {
    // Single instance is handled by tauri-plugin-single-instance in tauri.conf.json
    // This function can be used for additional logic if needed
    Ok(())
}