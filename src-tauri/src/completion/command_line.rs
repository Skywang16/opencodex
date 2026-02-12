//! Command line normalization and key extraction for completion learning.
//!
//! Goals:
//! - Extract "user's actual input command" from prompt/noise
//! - Generate stable small keys (control state space, ensure learning model size is manageable)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandKey {
    pub key: String,
    pub root: String,
    pub sub: Option<String>,
}

pub fn normalize_command_line(raw: &str) -> Option<String> {
    let line = raw.lines().last()?.trim();
    if line.is_empty() {
        return None;
    }

    // Common prompt formats: `user@host % cmd` / `user@host $ cmd`
    // Take the part after the last delimiter to avoid treating prompt as command.
    let mut best_start: Option<usize> = None;
    for delim in [" % ", " $ ", " # ", " > "] {
        if let Some(idx) = line.rfind(delim) {
            best_start =
                Some(best_start.map_or(idx + delim.len(), |prev| prev.max(idx + delim.len())));
        }
    }

    if let Some(start) = best_start {
        let cmd = line[start..].trim();
        if !cmd.is_empty() {
            return Some(cmd.to_string());
        }
    }

    // Fallback: when there's no space delimiter, try truncating at the last prompt character
    for ch in ['%', '$', '#', '>'] {
        if let Some(idx) = line.rfind(ch) {
            let cmd = line[idx + 1..].trim();
            if !cmd.is_empty() {
                return Some(cmd.to_string());
            }
        }
    }

    Some(line.to_string())
}

pub fn extract_command_key(raw_command_line: &str) -> Option<CommandKey> {
    let command_line = normalize_command_line(raw_command_line)?;
    let cleaned = strip_leading_noise(&command_line);
    let tokens = tokenize_simple(&cleaned);
    if tokens.is_empty() {
        return None;
    }

    let root = tokens[0].to_string();
    let sub = extract_subcommand(&root, &tokens);
    let key = match &sub {
        Some(sub) => format!("{root} {sub}"),
        None => root.clone(),
    };

    Some(CommandKey { key, root, sub })
}

fn strip_leading_noise(command_line: &str) -> String {
    let mut line = command_line.trim();

    // Common: sudo
    if let Some(rest) = line.strip_prefix("sudo ") {
        line = rest.trim_start();
    }

    // Environment variable assignment prefix: FOO=bar cmd
    // Only do the most conservative handling: consecutive `NAME=...` where NAME doesn't contain '/'.
    loop {
        let Some(first) = line.split_whitespace().next() else {
            break;
        };
        let Some(eq_pos) = first.find('=') else { break };
        let name = &first[..eq_pos];
        if name.is_empty() || name.contains('/') {
            break;
        }

        if let Some(rest) = line.strip_prefix(first) {
            line = rest.trim_start();
            continue;
        }
        break;
    }

    line.to_string()
}

fn tokenize_simple(command_line: &str) -> Vec<&str> {
    command_line.split_whitespace().collect()
}

fn extract_subcommand(root: &str, tokens: &[&str]) -> Option<String> {
    // Control state space: only take the second token for commands with "explicit subcommand semantics"
    // and skip options starting with '-'.
    if tokens.len() < 2 {
        return None;
    }

    let takes_sub = matches!(
        root,
        "git"
            | "docker"
            | "kubectl"
            | "cargo"
            | "npm"
            | "pnpm"
            | "yarn"
            | "go"
            | "brew"
            | "systemctl"
            | "journalctl"
    );
    if !takes_sub {
        return None;
    }

    let candidate = tokens[1];
    if candidate.starts_with('-') {
        return None;
    }

    Some(candidate.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_prompt_command_line() {
        let raw = "user@host % git status";
        assert_eq!(normalize_command_line(raw).as_deref(), Some("git status"));
    }

    #[test]
    fn extracts_git_subcommand_key() {
        let key = extract_command_key("git status").unwrap();
        assert_eq!(key.key, "git status");
        assert_eq!(key.root, "git");
        assert_eq!(key.sub.as_deref(), Some("status"));
    }

    #[test]
    fn extracts_non_subcommand_root_only() {
        let key = extract_command_key("ls -la").unwrap();
        assert_eq!(key.key, "ls");
        assert_eq!(key.root, "ls");
        assert_eq!(key.sub, None);
    }

    #[test]
    fn strips_sudo_prefix() {
        let key = extract_command_key("sudo git status").unwrap();
        assert_eq!(key.key, "git status");
    }
}
