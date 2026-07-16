use tauri::{Manager, WindowEvent};

#[cfg(target_os = "macos")]
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt,
};

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(OverlayPanel {
        config: {
            // Don't steal focus from the frontmost app / text field.
            can_become_key_window: false,
            can_become_main_window: false,
            is_floating_panel: true
        }
    })
}

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
fn apply_overlay_click_through(window: &tauri::WebviewWindow) {
    use objc2_app_kit::NSWindow;

    let _ = window.set_ignore_cursor_events(true);

    if let Ok(ns_window_ptr) = window.ns_window() {
        // SAFETY: ns_window pointer comes from Tauri's AppKit window handle.
        let ns_window = unsafe { &*(ns_window_ptr as *const NSWindow) };
        ns_window.setIgnoresMouseEvents(true);
    }
}

#[cfg(target_os = "macos")]
fn size_overlay_to_all_monitors(window: &tauri::WebviewWindow) {
    use tauri::{PhysicalPosition, PhysicalSize};

    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for monitor in monitors {
        let pos = monitor.position();
        let size = monitor.size();
        min_x = min_x.min(pos.x);
        min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x + size.width as i32);
        max_y = max_y.max(pos.y + size.height as i32);
    }

    let width = (max_x - min_x).max(1) as u32;
    let height = (max_y - min_y).max(1) as u32;
    let _ = window.set_position(PhysicalPosition::new(min_x, min_y));
    let _ = window.set_size(PhysicalSize::new(width, height));
}

/// macOS only shows overlays over *other apps'* fullscreen Spaces if the
/// window is a real NSPanel (not a normal NSWindow). Flags alone are not enough.
#[cfg(target_os = "macos")]
fn configure_macos_overlay(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    use objc2_app_kit::{NSColor, NSWindow};

    let Ok(ns_window_ptr) = window.ns_window() else {
        return Ok(());
    };

    // SAFETY: ns_window pointer comes from Tauri's AppKit window handle.
    let ns_window = unsafe { &*(ns_window_ptr as *const NSWindow) };
    ns_window.setOpaque(false);
    ns_window.setBackgroundColor(Some(&NSColor::clearColor()));

    size_overlay_to_all_monitors(window);

    let _ = window.set_resizable(false);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_always_on_top(true);

    // Convert NSWindow -> NSPanel (required for fullscreen Spaces).
    let panel = window.to_panel::<OverlayPanel>()?;

    panel.set_level(PanelLevel::Status.value());
    panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .stationary()
            .ignores_cycle()
            .into(),
    );
    panel.set_hides_on_deactivate(false);
    panel.show();

    apply_overlay_click_through(window);
    window.close_devtools();

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
            #[cfg(target_os = "macos")]
            WindowEvent::Resized(_)
            | WindowEvent::Moved(_)
            | WindowEvent::ScaleFactorChanged { .. } => {
                if window.label() == "overlay" {
                    if let Some(overlay) = window.app_handle().get_webview_window("overlay") {
                        apply_overlay_click_through(&overlay);
                    }
                }
            }
            #[cfg(target_os = "macos")]
            WindowEvent::Focused(_) => {
                if window.label() == "main" {
                    if let Some(webview) = window.app_handle().get_webview_window("main") {
                        configure_macos_dashboard(&webview);
                    }
                }
                if let Some(overlay) = window.app_handle().get_webview_window("overlay") {
                    apply_overlay_click_through(&overlay);
                }
            }
            _ => {}
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("overlay") {
                    configure_macos_overlay(&window)?;
                }
                if let Some(window) = app.get_webview_window("main") {
                    configure_macos_dashboard(&window);
                    let _ = window.set_focus();
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
                if !has_visible_windows {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });
}
