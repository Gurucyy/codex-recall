use crate::path_utils::{home_dir_string, username_guess};

pub fn redact_text(input: &str, enabled: bool) -> String {
    if !enabled {
        return input.to_string();
    }

    let mut output = input.to_string();
    if let Some(home) = home_dir_string() {
        output = output.replace(&home, "~");
    }
    if let Some(username) = username_guess() {
        output = output.replace(&format!("/Users/{username}"), "/Users/<user>");
        output = output.replace(&format!("\\Users\\{username}"), "\\Users\\<user>");
    }
    redact_secret_like_values(&output)
}

pub fn redact_secret_like_values(input: &str) -> String {
    let mut output = Vec::new();
    for token in input.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        if lower.contains("api_key=")
            || lower.contains("apikey=")
            || lower.contains("token=")
            || lower.contains("password=")
            || lower.starts_with("sk-")
        {
            output.push("<redacted-secret>".to_string());
        } else {
            output.push(token.to_string());
        }
    }
    output.join(" ")
}

pub fn truncate_long_blocks(input: &str, max_chars: Option<usize>) -> String {
    let Some(max_chars) = max_chars else {
        return input.to_string();
    };
    if input.chars().count() <= max_chars {
        input.to_string()
    } else {
        let mut value = input.chars().take(max_chars).collect::<String>();
        value.push_str("\n[truncated]");
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_secret_like_tokens() {
        assert_eq!(
            redact_text("token=abc hello", true),
            "<redacted-secret> hello"
        );
    }
}
