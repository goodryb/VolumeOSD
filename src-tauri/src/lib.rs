use tauri::{AppHandle, Manager, Emitter, menu::{Menu, MenuItem, CheckMenuItem}, tray::TrayIconBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code, ShortcutState};
use tauri_plugin_autostart::ManagerExt;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use volumecontrol::AudioDevice;

#[derive(Clone, Serialize)]
struct VolumePayload {
    volume: u8,
    muted: bool,
}

fn apply_volume_change(app: &AppHandle, delta: i32) {
    if let Ok(device) = AudioDevice::from_default() {
        // 如果当前是静音状态，先取消静音
        let is_muted = device.is_mute().unwrap_or(false);
        if is_muted {
            let _ = device.set_mute(false);
        }
        
        let current_vol = device.get_vol().unwrap_or(0) as i32;
        let new_vol = (current_vol + delta).clamp(0, 100) as u8;
        
        let _ = device.set_vol(new_vol);
        let final_muted = device.is_mute().unwrap_or(false);
        
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("volume-changed", VolumePayload {
                volume: new_vol,
                muted: final_muted,
            });
            let _ = window.center();
            let _ = window.show();
        }
    }
}

fn toggle_mute(app: &AppHandle) {
    if let Ok(device) = AudioDevice::from_default() {
        let is_muted = device.is_mute().unwrap_or(false);
        let _ = device.set_mute(!is_muted);
        let current_vol = device.get_vol().unwrap_or(0);
        
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("volume-changed", VolumePayload {
                volume: current_vol,
                muted: !is_muted,
            });
            let _ = window.show();
        }
    }
}

fn is_key_down(v_key: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(v_key.0 as i32) as u16 & 0x8000) != 0 }
}

static IS_ADJUSTING: AtomicBool = AtomicBool::new(false);

fn handle_hold(app: AppHandle, delta: i32, key: VIRTUAL_KEY) {
    if IS_ADJUSTING.load(Ordering::SeqCst) { return; }
    
    thread::spawn(move || {
        IS_ADJUSTING.store(true, Ordering::SeqCst);
        apply_volume_change(&app, delta);
        
        // 等待一小段时间后再检测是否长按，避免短按触发多次
        thread::sleep(Duration::from_millis(300));
        
        // 只有在延迟后仍然按住快捷键时才继续调整
        if is_key_down(key) && is_key_down(VK_CONTROL) && is_key_down(VK_MENU) {
            while is_key_down(key) && is_key_down(VK_CONTROL) && is_key_down(VK_MENU) {
                apply_volume_change(&app, delta);
                thread::sleep(Duration::from_millis(150));
            }
        }
        
        IS_ADJUSTING.store(false, Ordering::SeqCst);
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ctrl_alt_up = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowUp);
    let ctrl_alt_down = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowDown);
    let ctrl_alt_left = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::ArrowLeft);

    let ctrl_alt_up_clone = ctrl_alt_up.clone();
    let ctrl_alt_down_clone = ctrl_alt_down.clone();
    let ctrl_alt_left_clone = ctrl_alt_left.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--silent"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if shortcut == &ctrl_alt_up_clone {
                            handle_hold(app.clone(), 5, VK_UP);
                        } else if shortcut == &ctrl_alt_down_clone {
                            handle_hold(app.clone(), -5, VK_DOWN);
                        } else if shortcut == &ctrl_alt_left_clone {
                            toggle_mute(app);
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let _ = app.global_shortcut().register(ctrl_alt_up);
            let _ = app.global_shortcut().register(ctrl_alt_down);
            let _ = app.global_shortcut().register(ctrl_alt_left);

            // 禁用 WebView2 默认右键菜单，避免透明 OSD 窗口上右键弹出浏览器菜单
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.with_webview(|webview| unsafe {
                    let _ = webview
                        .controller()
                        .CoreWebView2()
                        .and_then(|core| core.Settings())
                        .and_then(|settings| settings.SetAreDefaultContextMenusEnabled(false));
                });
            }

            // Tray menu
            let version_text = format!("版本 {}", env!("CARGO_PKG_VERSION"));
            let version_i = MenuItem::with_id(app, "version", &version_text, false, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let mute_i = MenuItem::with_id(app, "mute", "静音/取消静音", true, None::<&str>)?;
            
            // Autostart toggle
            let is_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let autostart_i = CheckMenuItem::with_id(app, "autostart", "开机启动", true, is_enabled, None::<&str>)?;
            
            let autostart_i_handle = autostart_i.clone();
            let menu = Menu::with_items(app, &[&autostart_i, &version_i, &mute_i, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false);
            
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }

            let _tray = tray_builder
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "quit" => { app.exit(0); }
                        "mute" => { toggle_mute(app); }
                        "autostart" => {
                            let is_enabled = app.autolaunch().is_enabled().unwrap_or(false);
                            if is_enabled {
                                match app.autolaunch().disable() {
                                    Ok(_) => { let _ = autostart_i_handle.set_checked(false); },
                                    Err(e) => eprintln!("Failed to disable autostart: {}", e),
                                }
                            } else {
                                match app.autolaunch().enable() {
                                    Ok(_) => { let _ = autostart_i_handle.set_checked(true); },
                                    Err(e) => eprintln!("Failed to enable autostart: {}", e),
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
