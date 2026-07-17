use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::mic_sound;
use crate::TapErrorBounds;

static DICTATION_ACTIVE: AtomicBool = AtomicBool::new(false);
static TAP_ERROR_DISMISS_WATCH: AtomicBool = AtomicBool::new(false);
static TAP_ERROR_BOUNDS: Mutex<Option<TapErrorBounds>> = Mutex::new(None);
static PRESS_START: Mutex<Option<Instant>> = Mutex::new(None);

const MIN_HOLD_MS: u128 = 500;

impl TapErrorBounds {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

pub fn set_tap_error_click_dismiss_watch(watching: bool, bounds: Option<TapErrorBounds>) {
    TAP_ERROR_DISMISS_WATCH.store(watching, Ordering::SeqCst);

    if let Ok(mut stored_bounds) = TAP_ERROR_BOUNDS.lock() {
        *stored_bounds = if watching { bounds } else { None };
    }
}

fn emit_dictation_state(app: &AppHandle, active: bool) {
    let was_active = DICTATION_ACTIVE.swap(active, Ordering::SeqCst);
    if was_active == active {
        return;
    }

    if active {
        if let Ok(mut press_start) = PRESS_START.lock() {
            *press_start = Some(Instant::now());
        }

        let _ = app.emit("dictation:state", serde_json::json!({ "active": true }));
        return;
    }

    if !was_active {
        return;
    }

    let held_ms = PRESS_START
        .lock()
        .ok()
        .and_then(|mut press_start| press_start.take())
        .map(|started| started.elapsed().as_millis())
        .unwrap_or(0);
    let too_short = held_ms < MIN_HOLD_MS;

    if !too_short {
        mic_sound::play();
    }

    let _ = app.emit(
        "dictation:state",
        serde_json::json!({ "active": false, "tooShort": too_short }),
    );
}

pub fn start(app: AppHandle) {
    mic_sound::init();

    std::thread::spawn(move || {
        use core_foundation::runloop::CFRunLoop;
        use core_graphics::event::{
            CallbackResult, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
            CGEventTapPlacement, CGEventType,
        };

        let app_for_callback = app.clone();
        let result = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::FlagsChanged,
                CGEventType::LeftMouseDown,
                CGEventType::RightMouseDown,
            ],
            move |_proxy, event_type, event| {
                match event_type {
                    CGEventType::FlagsChanged => {
                        let flags = event.get_flags();
                        let ctrl = flags.contains(CGEventFlags::CGEventFlagControl);
                        let option = flags.contains(CGEventFlags::CGEventFlagAlternate);
                        emit_dictation_state(&app_for_callback, ctrl && option);
                    }
                    CGEventType::LeftMouseDown | CGEventType::RightMouseDown => {
                        if !TAP_ERROR_DISMISS_WATCH.load(Ordering::Relaxed) {
                            return CallbackResult::Keep;
                        }

                        let location = event.location();
                        let click_x = location.x as f64;
                        let click_y = location.y as f64;

                        let inside_modal = TAP_ERROR_BOUNDS
                            .lock()
                            .ok()
                            .and_then(|bounds| bounds.as_ref().copied())
                            .is_some_and(|bounds| bounds.contains(click_x, click_y));

                        if !inside_modal {
                            let _ = app_for_callback.emit("taperror:dismiss", ());
                        }
                    }
                    _ => {}
                }

                CallbackResult::Keep
            },
            CFRunLoop::run_current,
        );

        if result.is_err() {
            eprintln!(
                "LocalFlow: failed to create modifier event tap. \
                 Grant Accessibility and Input Monitoring, then restart."
            );
        }
    });
}
