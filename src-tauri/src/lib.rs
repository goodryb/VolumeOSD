use tauri::{AppHandle, Manager, Emitter, menu::{Menu, MenuItem, CheckMenuItem}, tray::TrayIconBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code, ShortcutState};
use tauri_plugin_autostart::ManagerExt;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::*,
    Win32::Media::Audio::Endpoints::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
    Win32::UI::Input::KeyboardAndMouse::*,
};

#[derive(Clone, Serialize)]
struct VolumePayload {
    volume: u8,
    muted: bool,
}

fn get_current_volume_int(volume_interface: &IAudioEndpointVolume) -> i32 {
    unsafe {
        let vol_f = volume_interface.GetMasterVolumeLevelScalar().unwrap_or(0.0);
        (vol_f * 100.0).round() as i32
    }
}

fn apply_volume_change(volume_interface: &IAudioEndpointVolume, app: &AppHandle, delta: i32) {
    unsafe {
        let current_int = get_current_volume_int(volume_interface);
        let mut new_vol_int = current_int + delta;
        if delta.abs() == 5 {
             new_vol_int = (new_vol_int / 5) * 5;
        }
        new_vol_int = new_vol_int.clamp(0, 100);
        
        let _ = volume_interface.SetMasterVolumeLevelScalar(new_vol_int as f32 / 100.0, std::ptr::null());
        let is_muted = volume_interface.GetMute().unwrap_or(false.into()).as_bool();
        
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("volume-changed", VolumePayload {
                volume: new_vol_int as u8,
                muted: is_muted,
            });
            let _ = window.center();
            let _ = window.show();
        }
    }
}

fn get_volume_interface() -> Result<IAudioEndpointVolume> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
    }
}

fn toggle_mute(app: &AppHandle) {
    if let Ok(volume_interface) = get_volume_interface() {
        unsafe {
            let is_muted = volume_interface.GetMute().unwrap_or(false.into()).as_bool();
            let _ = volume_interface.SetMute(!is_muted, std::ptr::null());
            let current_vol = get_current_volume_int(&volume_interface) as u8;
            
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("volume-changed", VolumePayload {
                    volume: current_vol,
                    muted: !is_muted,
                });
                let _ = window.show();
            }
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
        if let Ok(volume_interface) = get_volume_interface() {
            apply_volume_change(&volume_interface, &app, delta);
            thread::sleep(Duration::from_millis(400));
            
            while is_key_down(key) && is_key_down(VK_CONTROL) && is_key_down(VK_MENU) {
                apply_volume_change(&volume_interface, &app, delta);
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

            // Tray menu
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let mute_i = MenuItem::with_id(app, "mute", "静音/取消静音", true, None::<&str>)?;
            
            // Autostart toggle
            let is_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let autostart_i = CheckMenuItem::with_id(app, "autostart", "开机启动", true, is_enabled, None::<&str>)?;
            
            let autostart_i_handle = autostart_i.clone();
            let menu = Menu::with_items(app, &[&autostart_i, &mute_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "quit" => { app.exit(0); }
                        "mute" => { toggle_mute(app); }
                        "autostart" => {
                            // 直接查询插件的真实状态
                            let is_enabled = app.autolaunch().is_enabled().unwrap_or(false);
                            if is_enabled {
                                match app.autolaunch().disable() {
                                    Ok(_) => {
                                        let _ = autostart_i_handle.set_checked(false);
                                        println!("Autostart disabled successfully.");
                                    },
                                    Err(e) => eprintln!("Failed to disable autostart: {}", e),
                                }
                            } else {
                                match app.autolaunch().enable() {
                                    Ok(_) => {
                                        let _ = autostart_i_handle.set_checked(true);
                                        println!("Autostart enabled successfully.");
                                    },
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
