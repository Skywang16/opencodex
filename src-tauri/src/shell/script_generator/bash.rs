//! Bash integration script generator

use super::ShellIntegrationConfig;

/// Node.js version detection script (compatible with Bash and Zsh)
const NODE_VERSION_DETECTION: &str = r#"
    # Node.js version detection
    __opencodex_last_node_version=""

    __opencodex_detect_node_version() {
        if command -v node >/dev/null 2>&1; then
            local current_version=$(node -v 2>/dev/null | tr -d '\n')
            if [[ -n "$current_version" && "$current_version" != "$__opencodex_last_node_version" ]]; then
                __opencodex_last_node_version="$current_version"
                printf '\e]1337;OpenCodexNodeVersion=%s\e\\' "$current_version"
            fi
        elif [[ -n "$__opencodex_last_node_version" ]]; then
            __opencodex_last_node_version=""
            printf '\e]1337;OpenCodexNodeVersion=\e\\'
        fi
    }
"#;

/// Generate Bash integration script
pub fn generate_script(config: &ShellIntegrationConfig) -> String {
    let mut script = String::new();

    script.push_str(
        r#"
# OpenCodex Integration Start
if [[ -z "$OPENCODEX_SHELL_INTEGRATION" ]]; then
    export OPENCODEX_SHELL_INTEGRATION=1
    export OPENCODEX_INTEGRATION_LOADED=1

    # Original PS1 backup
    if [[ -z "$OPENCODEX_ORIGINAL_PS1" ]]; then
        export OPENCODEX_ORIGINAL_PS1="$PS1"
    fi
"#,
    );

    // Add Node version detection function
    script.push_str(NODE_VERSION_DETECTION);

    // Only add related functions when command tracking is enabled (using standard OSC 133 markers)
    if config.enable_command_tracking {
        script.push_str(
            r#"
    # Shell Integration support - OSC 133 markers
    __opencodex_preexec() {
        # C: Command execution start, carries command content
        # $1 is BASH_COMMAND passed through DEBUG trap
        printf '\e]133;C;%s\e\\' "$1" >/dev/tty
    }

    __opencodex_precmd() {
        local exit_code=$?
        # D: Command finished, includes exit code
        printf '\e]133;D;%d\e\\' "$exit_code" >/dev/tty
        # A: Prompt start
        printf '\e]133;A\e\\' >/dev/tty
        # B: Command input area start
        printf '\e]133;B\e\\' >/dev/tty
        __opencodex_detect_node_version
    }
"#,
        );
    }

    // Add command tracking functionality: through DEBUG trap and PROMPT_COMMAND (common Bash practice)
    if config.enable_command_tracking {
        script.push_str(
            r#"
    if [[ -z "$OPENCODEX_PREEXEC_INSTALLED" ]]; then
        export OPENCODEX_PREEXEC_INSTALLED=1

        # Use DEBUG trap to simulate preexec
        trap '__opencodex_preexec "$BASH_COMMAND"' DEBUG

        # Run precmd before prompt rendering
        if [[ -z "$PROMPT_COMMAND" ]]; then
            PROMPT_COMMAND="__opencodex_precmd"
        else
            PROMPT_COMMAND="$PROMPT_COMMAND; __opencodex_precmd"
        fi
    fi
"#,
        );
    }

    // (CWD sync alias removed, relies on standard OSC 7 handled by other logic, can be implemented separately if needed)

    // (Window title update removed to keep minimal implementation, can use standard OSC 2 separately if needed)

    // Add custom environment variables
    if !config.custom_env_vars.is_empty() {
        script.push_str("\n    # Custom environment variables\n");
        for (key, value) in &config.custom_env_vars {
            script.push_str(&format!("    export {key}=\"{value}\"\n"));
        }
    }

    // If command tracking is not enabled, need to set up Node version detection separately
    if !config.enable_command_tracking {
        script.push_str(
            r#"
    # Node version detection (when command tracking is disabled)
    if [[ -z "$PROMPT_COMMAND" ]]; then
        PROMPT_COMMAND="__opencodex_detect_node_version"
    else
        PROMPT_COMMAND="$PROMPT_COMMAND; __opencodex_detect_node_version"
    fi
"#,
        );
    }

    script.push_str(
        r#"
    # Immediately detect Node version on initialization
    __opencodex_detect_node_version 2>/dev/null || true

fi
# OpenCodex Integration End
"#,
    );

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_basic_bash_script_generation() {
        let config = ShellIntegrationConfig::default();
        let script = generate_script(&config);

        assert!(script.contains("# OpenCodex Integration Start"));
        assert!(script.contains("# OpenCodex Integration End"));
        assert!(script.contains("OPENCODEX_INTEGRATION_LOADED"));
        assert!(script.contains("__opencodex_detect_node_version"));
    }

    #[test]
    fn test_command_tracking_enabled() {
        let config = ShellIntegrationConfig {
            enable_command_tracking: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__opencodex_preexec"));
        assert!(script.contains("__opencodex_precmd"));
        assert!(script.contains("PROMPT_COMMAND"));
        assert!(script.contains("trap '__opencodex_preexec"));
    }

    #[test]
    fn test_cwd_sync_enabled() {
        let config = ShellIntegrationConfig {
            enable_cwd_sync: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        // After simplification, CWD is not synced via alias, but should still include node version detection logic
        assert!(script.contains("__opencodex_detect_node_version"));
    }

    #[test]
    fn test_title_updates_enabled() {
        let config = ShellIntegrationConfig {
            enable_title_updates: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        // After simplification, window title is not updated, but should still include node version detection logic
        assert!(script.contains("__opencodex_detect_node_version"));
    }

    #[test]
    fn test_custom_env_vars() {
        let mut custom_vars = HashMap::new();
        custom_vars.insert("OPENCODEX_CUSTOM".to_string(), "test_value".to_string());
        custom_vars.insert("ANOTHER_VAR".to_string(), "another_value".to_string());

        let config = ShellIntegrationConfig {
            custom_env_vars: custom_vars,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("export OPENCODEX_CUSTOM=\"test_value\""));
        assert!(script.contains("export ANOTHER_VAR=\"another_value\""));
    }

    #[test]
    fn test_all_features_disabled() {
        let config = ShellIntegrationConfig {
            enable_command_tracking: false,
            enable_cwd_sync: false,
            enable_title_updates: false,
            custom_env_vars: HashMap::new(),
        };
        let script = generate_script(&config);

        // Should still contain basic structure
        assert!(script.contains("# OpenCodex Integration Start"));
        assert!(script.contains("# OpenCodex Integration End"));
        assert!(script.contains("OPENCODEX_INTEGRATION_LOADED"));

        // Should not contain disabled features
        assert!(!script.contains("opencodex_preexec"));
        assert!(!script.contains("opencodex_cd"));
        assert!(!script.contains("opencodex_update_title"));
    }
}
