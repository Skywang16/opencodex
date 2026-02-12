/// Command sequence prediction - hardcoded association table
///
pub static COMMAND_PAIRS: &[(&str, &[&str])] = &[
    // Network debugging workflow
    ("lsof", &["kill", "kill -9", "netstat"]),
    ("netstat", &["kill", "lsof", "ss"]),
    ("ss", &["kill", "lsof"]),
    // Docker workflow
    (
        "docker ps",
        &["docker stop", "docker logs", "docker exec", "docker rm"],
    ),
    (
        "docker images",
        &["docker rmi", "docker run", "docker pull"],
    ),
    ("docker logs", &["docker restart", "docker stop"]),
    // Git workflow
    ("git status", &["git add", "git diff", "git restore"]),
    ("git add", &["git commit", "git status", "git reset"]),
    ("git commit", &["git push", "git log", "git show"]),
    ("git pull", &["git status", "git log"]),
    ("git diff", &["git add", "git restore"]),
    // Process management
    ("ps aux", &["kill", "kill -9", "pkill"]),
    ("top", &["kill", "pkill"]),
    ("htop", &["kill", "pkill"]),
    // File search
    ("find", &["xargs", "rm", "ls", "cat"]),
    ("grep", &["cat", "less", "vim", "code"]),
    ("ls", &["cd", "cat", "less", "rm", "mv", "cp"]),
    // Package management - Node
    ("npm install", &["npm run", "npm start", "npm test"]),
    ("npm run", &["npm test", "git add"]),
    ("npm test", &["git add", "npm run"]),
    // Package management - Python
    ("pip install", &["python", "pytest"]),
    ("pytest", &["git add", "python"]),
    // Package management - Rust
    (
        "cargo build",
        &["cargo run", "cargo test", "./target/debug"],
    ),
    ("cargo test", &["git add", "cargo build"]),
    ("cargo run", &["cargo build", "cargo test"]),
    // System operations
    (
        "systemctl status",
        &["systemctl restart", "systemctl stop", "journalctl"],
    ),
    ("journalctl", &["systemctl restart", "systemctl status"]),
];

/// Check if command matches a pattern
pub fn matches_command_pattern(executed_cmd: &str, pattern: &str) -> bool {
    // Only match "command word boundaries" to avoid false matches like `ls` matching `lsblk`.
    //
    // Rules:
    // - If pattern is 1 word: match the 1st word of executed command
    // - If pattern is N words: match the first N words of executed command
    // - Allow `sudo` prefix (common real-world scenario)
    let mut cmd = executed_cmd.trim();
    if let Some(rest) = cmd.strip_prefix("sudo ") {
        cmd = rest.trim_start();
    }

    let pattern_tokens: Vec<&str> = pattern.split_whitespace().collect();
    if pattern_tokens.is_empty() {
        return false;
    }

    let cmd_tokens: Vec<&str> = cmd.split_whitespace().collect();
    if cmd_tokens.len() < pattern_tokens.len() {
        return false;
    }

    // Safe slicing
    let head = cmd_tokens
        .get(..pattern_tokens.len())
        .map(|tokens| tokens.join(" "))
        .unwrap_or_default();
    head == pattern
}

/// Get suggested next commands
pub fn get_suggested_commands(last_command: &str) -> Option<Vec<String>> {
    for (pattern, suggestions) in COMMAND_PAIRS {
        if matches_command_pattern(last_command, pattern) {
            return Some(suggestions.iter().map(|s| s.to_string()).collect());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsof_kill_prediction() {
        let suggestions = get_suggested_commands("lsof -i :8080");
        assert!(suggestions.is_some());
        let cmds = suggestions.unwrap();
        assert!(cmds.contains(&"kill".to_string()));
        assert!(cmds.contains(&"kill -9".to_string()));
    }

    #[test]
    fn test_git_workflow() {
        // git status → git add
        let suggestions = get_suggested_commands("git status");
        assert!(suggestions.is_some());
        assert!(suggestions.unwrap().contains(&"git add".to_string()));

        // git add → git commit
        let suggestions = get_suggested_commands("git add src/main.rs");
        assert!(suggestions.is_some());
        assert!(suggestions.unwrap().contains(&"git commit".to_string()));
    }

    #[test]
    fn test_docker_workflow() {
        let suggestions = get_suggested_commands("docker ps -a");
        assert!(suggestions.is_some());
        let cmds = suggestions.unwrap();
        assert!(cmds.contains(&"docker stop".to_string()));
        assert!(cmds.contains(&"docker logs".to_string()));
    }

    #[test]
    fn test_no_match() {
        let suggestions = get_suggested_commands("echo hello");
        assert!(suggestions.is_none());
    }
}
