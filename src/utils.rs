use regex::Regex;

use crate::errors::ScriptErrors;

pub fn capitalize_word(s: &str) -> Result<String, ScriptErrors> {
    let start = s
        .chars()
        .next()
        .ok_or(ScriptErrors::ConfigError(format!("Empty string")))?;
    let rest = s.chars().skip(1).collect::<String>();
    Ok(format!("{}{}", start.to_uppercase(), rest))
}

pub fn remove_ansi_codes(s: &str) -> String {
    // Regex to match ANSI escape codes
    let re = Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]")
        .unwrap_or_else(|e| panic!("Failed to compile regex to remove ansi codes: {}", e));
    re.replace_all(s, "").to_string()
}
