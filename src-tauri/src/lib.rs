use tauri::{Manager, WindowEvent};

#[cfg(target_os = "macos")]
fn configure_macos_window(window: &tauri::WebviewWindow) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // macOS-style: close hides the window; app stays running in the Dock.
                let _ = window.hide();
                api.prevent_close();
            }
            #[cfg(target_os = "macos")]
            WindowEvent::Focused(_) => {
                if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
                    configure_macos_window(&webview);
                }
            }
            _ => {}
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                configure_macos_window(&window);
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
