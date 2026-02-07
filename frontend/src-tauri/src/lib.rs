use std::collections::HashMap;
use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct QuickCaptureShortcutBinding {
    shortcut: &'static str,
    mode: &'static str,
    target: &'static str,
}

const QUICK_CAPTURE_SHORTCUT_BINDINGS: [QuickCaptureShortcutBinding; 6] = [
    QuickCaptureShortcutBinding {
        shortcut: "CommandOrControl+Shift+N",
        mode: "task",
        target: "default",
    },
    QuickCaptureShortcutBinding {
        shortcut: "CommandOrControl+Shift+M",
        mode: "note",
        target: "default",
    },
    QuickCaptureShortcutBinding {
        shortcut: "CommandOrControl+Shift+L",
        mode: "link",
        target: "default",
    },
    QuickCaptureShortcutBinding {
        shortcut: "CommandOrControl+Shift+V",
        mode: "voice",
        target: "default",
    },
    QuickCaptureShortcutBinding {
        shortcut: "CommandOrControl+Shift+I",
        mode: "task",
        target: "inbox",
    },
    QuickCaptureShortcutBinding {
        shortcut: "CommandOrControl+Shift+D",
        mode: "note",
        target: "daily",
    },
];

fn quick_capture_event_script(mode: &str, target: &str) -> String {
    format!(
        "window.dispatchEvent(new CustomEvent('mindvault:quick-capture', {{ detail: {{ mode: '{mode}', target: '{target}' }} }}));"
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut_modes: Arc<HashMap<String, (&'static str, &'static str)>> = Arc::new(
        QUICK_CAPTURE_SHORTCUT_BINDINGS
            .iter()
            .filter_map(|binding| {
                binding
                    .shortcut
                    .parse::<Shortcut>()
                    .ok()
                    .map(|shortcut| (shortcut.to_string(), (binding.mode, binding.target)))
            })
            .collect(),
    );
    let handler_shortcut_modes = Arc::clone(&shortcut_modes);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        let (mode, target) = handler_shortcut_modes
                            .get(&shortcut.to_string())
                            .copied()
                            .unwrap_or(("task", "default"));
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                        let _ = window.eval(&quick_capture_event_script(mode, target));
                    }
                })
                .build(),
        )
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            for binding in QUICK_CAPTURE_SHORTCUT_BINDINGS {
                match binding.shortcut.parse::<Shortcut>() {
                    Ok(shortcut) => {
                        if let Err(err) = app.global_shortcut().register(shortcut) {
                            log::warn!(
                                "Failed to register global shortcut {}: {err}",
                                binding.shortcut
                            );
                        }
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to parse global shortcut {}: {err}",
                            binding.shortcut
                        );
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
