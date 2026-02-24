pub mod agent;
pub mod ai;
pub mod checkpoint;
pub mod code_intel;
pub mod commands;
pub mod config;
pub mod events;
pub mod file_watcher;
pub mod filesystem;
pub mod git;
pub mod llm;
pub mod menu;
pub mod mux;
pub mod node;
pub mod settings;
pub mod setup;
pub mod shell;
pub mod storage;
pub mod terminal;
pub mod utils;
pub mod vector_db;
pub mod window;
pub mod workspace;

use setup::{
    ensure_main_window_visible, handle_startup_args, init_logging, init_plugin,
    initialize_app_states, setup_app_events, setup_deep_links,
};
use utils::i18n::I18nManager;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    if let Err(e) = I18nManager::initialize() {
        eprintln!("Internationalization initialization failed: {e}");
    }

    let mut builder = tauri::Builder::default();

    // Configure single instance plugin (desktop platforms only)
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.len() > 1 {
                let file_path = &argv[1];
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("file-dropped", file_path);
                }
            }
        }));
    }

    let app_result = builder
        .plugin(init_plugin("init"))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin({
            #[cfg(target_os = "macos")]
            {
                tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::AppleScript,
                    None,
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                tauri_plugin_autostart::Builder::new().build()
            }
        });

    let app_result = commands::register_all_commands(app_result);

    let app_instance = app_result
        .setup(|app| {
            if let Err(e) = initialize_app_states(app) {
                eprintln!("Application initialization failed: {e}");
                std::process::exit(1);
            }

            // Create and set application menu
            match menu::create_menu(app.handle()) {
                Ok(menu) => {
                    if let Err(e) = app.set_menu(menu) {
                        eprintln!("Failed to set menu: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create menu: {e}");
                }
            }

            // Register menu event handler
            let app_handle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                menu::handle_menu_event(&app_handle, event.id().as_ref());
            });

            setup_app_events(app);
            setup_deep_links(app);
            handle_startup_args(app);
            ensure_main_window_visible(app);

            // macOS: Apply vibrancy effect
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                use window_vibrancy::{
                    apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState,
                };
                let _ = apply_vibrancy(
                    &window,
                    NSVisualEffectMaterial::HudWindow,
                    Some(NSVisualEffectState::Active),
                    None,
                );
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Error occurred while building Tauri application: {e}");
            std::process::exit(1);
        });

    app_instance.run(|app_handle, event| {
        match event {
            // macOS: Listen for application activation event (clicking Dock icon)
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                // User clicked Dock icon, check if main window is hidden
                if let Some(window) = app_handle.get_webview_window("main") {
                    // Check if window is visible
                    if let Ok(is_visible) = window.is_visible() {
                        if !is_visible {
                            // If window is hidden, show it again
                            if let Err(e) = window.show() {
                                eprintln!("Unable to show window: {e}");
                            }
                            // Bring window to front
                            let _ = window.set_focus();
                        }
                    }
                }
            }
            // Listen for application exit event (Command+Q or menu exit)
            // Clean up resources before app truly exits
            tauri::RunEvent::ExitRequested { .. } => {
                if let Err(e) = crate::mux::singleton::shutdown_mux() {
                    eprintln!("Failed to cleanup TerminalMux: {e}");
                }
            }
            _ => {}
        }
    });
}
