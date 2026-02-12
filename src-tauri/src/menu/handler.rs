use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_opener::OpenerExt;

const DOCS_URL: &str = "https://github.com/user/opencodex";
const ISSUES_URL: &str = "https://github.com/user/opencodex/issues";

/// Handle menu events
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        // Events forwarded to frontend
        "new_terminal"
        | "find"
        | "clear_terminal"
        | "toggle_terminal_panel"
        | "toggle_always_on_top"
        | "preferences" => {
            let _ = app.emit(&format!("menu:{}", event_id.replace('_', "-")), ());
        }

        // Help
        "documentation" => {
            let _ = app.opener().open_url(DOCS_URL, None::<&str>);
        }
        "report_issue" => {
            let _ = app.opener().open_url(ISSUES_URL, None::<&str>);
        }

        _ => {}
    }
}
