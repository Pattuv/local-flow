use tauri::{Manager, WindowEvent};

#[cfg(target_os = "macos")]
fn configure_macos_dashboard(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSWindow, NSWindowButton,
    };

    let _ = window.set_theme(Some(tauri::Theme::Light));

    let Ok(ns_window_ptr) = window.ns_window() else {
        return;
    };

    // SAFETY: ns_window pointer comes from Tauri's AppKit window handle.
    let ns_window = unsafe { &*(ns_window_ptr as *const NSWindow) };

    let appearance = unsafe { NSAppearance::appearanceNamed(NSAppearanceNameAqua) };
    if let Some(appearance) = appearance {
        ns_window.setAppearance(Some(&appearance));
    }

    // Keep traffic lights present (not auto-hidden) when the window is inactive.
    for button in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(btn) = ns_window.standardWindowButton(button) {
            btn.setHidden(false);
            btn.setAlphaValue(1.0);
        }
    }
}

#[cfg(target_os = "macos")]
fn configure_macos_overlay(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSColor, NSWindow};
    use tauri::{PhysicalPosition, PhysicalSize};

    let Ok(ns_window_ptr) = window.ns_window() else {
        return;
    };

    // SAFETY: ns_window pointer comes from Tauri's AppKit window handle.
    let ns_window = unsafe { &*(ns_window_ptr as *const NSWindow) };
    ns_window.setOpaque(false);
    ns_window.setBackgroundColor(Some(&NSColor::clearColor()));

    // Cover the entire current monitor (not exclusive fullscreen).
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let position = monitor.position();
        let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
        let _ = window.set_size(PhysicalSize::new(size.width, size.height));
    }

    let _ = window.set_resizable(false);
    let _ = window.set_skip_taskbar(true);
    // Full click-through: clicks pass to apps underneath.
    let _ = window.set_ignore_cursor_events(true);
    // Keep inspector closed if anything tries to open it.
    window.close_devtools();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // Only the dashboard uses macOS-style hide-on-close.
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
            #[cfg(target_os = "macos")]
            WindowEvent::Focused(_) => {
                if window.label() == "main" {
                    if let Some(webview) = window.app_handle().get_webview_window("main") {
                        configure_macos_dashboard(&webview);
                    }
                }
            }
            _ => {}
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    configure_macos_dashboard(&window);
                }
                if let Some(window) = app.get_webview_window("overlay") {
                    configure_macos_overlay(&window);
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                // Clicking the Dock icon re-shows a hidden window.
                if !has_visible_windows {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });
}
