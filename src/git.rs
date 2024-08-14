use std::fs;

use regex::Regex;

use crate::{
    config::Config,
    errors::ScriptErrors,
    utils::{capitalize_word, remove_ansi_codes},
};

pub fn get_pr_title(branch_name: &str, config: &Config) -> String {
    let parts: Vec<&str> = branch_name.split('-').collect();

    if parts.is_empty() {
        return branch_name.to_string();
    }

    let branch_start = parts[0];

    let join_remaining_parts = |parts: &[&str]| {
        parts
            .iter()
            .skip(1)
            .map(|&s| s)
            .collect::<Vec<&str>>()
            .join(" ")
    };

    if let Some(pr_title) = config.title.jira_prefixes.get(branch_start) {
        if parts.len() > 1 {
            let ticket_number = parts[1];
            let ticket_name = join_remaining_parts(&parts);
            return pr_title
                .replace("{ticket_number}", ticket_number)
                .replace("{ticket_name}", &ticket_name);
        }
    } else if let Some(pr_title) = config.title.prefixes.get(branch_start) {
        let replaced_title = branch_name
            .replacen(branch_start, pr_title, 1)
            .replace("-", " ");
        return replaced_title;
    }

    // Capitalize each word if no prefix was found
    parts
        .iter()
        .map(|&word| capitalize_word(word).unwrap_or(word.to_string()))
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn get_target_branch(branch_name: &String, config: &Config) -> String {
    config
        .branches
        .includes
        .iter()
        .find_map(|(key, value)| {
            if branch_name.to_lowercase().contains(key) {
                Some(value.clone())
            } else {
                None
            }
        })
        .unwrap_or(config.branches.default.clone())
}

fn normalize_commits(commits: &str, prefixes: &[String]) -> Result<Vec<String>, ScriptErrors> {
    // Construct the regex pattern for the prefixes with optional scope
    let pattern = prefixes
        .iter()
        .map(|prefix| {
            // Escape special characters in prefix and build the regex pattern
            format!(r"{}(?:\([^\)]*\))?:", prefix)
        })
        .collect::<Vec<String>>()
        .join("|");

    // Compile the regex pattern
    let re = Regex::new(&pattern)
        .map_err(|e| ScriptErrors::ConfigError(format!("Failed to compile regex: {}", e)))?;

    let mut replaced_commits: Vec<String> = commits
        .lines()
        .rev()
        .filter(|line| !line.is_empty())
        .filter(|line| re.is_match(line))
        .map(|line| {
            let matched_prefix = re
                .find(line)
                .unwrap_or_else(|| panic!("Failed to find prefix in commit message"))
                .as_str();
            // Get the real prefix instead of the regex capture group (removing the scope)
            let mut clean_prefix = String::new();
            for c in prefixes.iter() {
                if matched_prefix.contains(c) {
                    clean_prefix = c.to_string();
                    break;
                }
            }
            // The message is the rest of the line after the prefix, it's not captured
            let message = line.split(matched_prefix).nth(1).unwrap_or_default().trim();
            format!("{}: {}", clean_prefix, message)
        })
        .map(|line| remove_ansi_codes(&line))
        .collect();

    // sort by first their prefix
    replaced_commits.sort_by(|a, b| {
        let a_prefix = a.split(": ").next().unwrap_or_default();
        let b_prefix = b.split(": ").next().unwrap_or_default();
        a_prefix.cmp(b_prefix)
    });

    Ok(replaced_commits)
}

pub fn get_commit_body(config: &Config, target_branch: &String) -> Result<String, ScriptErrors> {
    let mut commit_body = match fs::read(config.template.path.clone()) {
        Ok(content) => String::from_utf8(content).map_err(|e| {
            ScriptErrors::ConfigError(format!(
                "Failed to parse template file '{}': {}",
                config.template.path, e
            ))
        })?,
        Err(e) => panic!(
            "Could not read template file '{}': {}",
            config.template.path, e
        ),
    };

    let output = std::process::Command::new("git")
        .args(&["log", "--oneline", &format!("{}..HEAD", target_branch)])
        .output()
        .map_err(|e| {
            ScriptErrors::ConfigError(format!("Failed to get commit list from git: {}", e))
        })?;

    let commits = String::from_utf8_lossy(&output.stdout);

    let relevant_commit_prefixes = config
        .commits
        .prefixes
        .keys()
        .cloned()
        .collect::<Vec<String>>();

    let commits_as_body = normalize_commits(&commits, &relevant_commit_prefixes)?
        .iter()
        .map(|line| {
            let matching_prefix = relevant_commit_prefixes
                .iter()
                .find(|prefix| line.contains(format!("{}:", prefix).as_str()))
                .unwrap_or_else(|| {
                    panic!("Could not find matching prefix in commit message: {}", line)
                });
            let prefix_config = config
                .commits
                .prefixes
                .get(matching_prefix)
                .unwrap_or_else(|| {
                    panic!(
                        "Could not find prefix config for prefix {}",
                        matching_prefix
                    )
                });
            let replaced = line
                .replacen(format!("{}:", matching_prefix).as_str(), prefix_config, 1)
                .trim()
                .to_string();

            let first_letter = &replaced.chars().next().unwrap_or_default();
            let capitalized = format!(
                "{}{}",
                first_letter.to_uppercase(),
                replaced.chars().skip(1).collect::<String>()
            );

            format!("- {}", capitalized)
        })
        .collect::<Vec<String>>()
        .join("\n");

    if !commit_body.contains("{LIST_COMMITS}") {
        return Err(ScriptErrors::ConfigError(
            format!("{}", "Commit body does not contain {LIST_COMMITS}").to_string(),
        ));
    }

    commit_body = commit_body.replace("{LIST_COMMITS}", &commits_as_body);
    Ok(commit_body)
}

pub fn get_pr_labels(config: &Config, branch_name: &String) -> Vec<String> {
    let mut labels = config.labels.default.clone();

    for (key, value) in config.labels.includes.iter() {
        if branch_name.to_lowercase().contains(key) {
            labels.extend(value.clone());
        }
    }

    labels
}
