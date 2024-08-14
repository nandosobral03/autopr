use core::panic;
use serde::Deserialize;
use std::{collections::HashMap, env, fs};

#[derive(serde::Deserialize, Debug)]
pub enum ScriptErrors {
    ConfigError(String),
    ParseError(String),
}

fn capitalize_word(s: &str) -> Result<String, ScriptErrors> {
    let start = s
        .chars()
        .next()
        .ok_or(ScriptErrors::ConfigError(format!("Empty string")))?;
    let rest = s.chars().skip(1).collect::<String>();
    Ok(format!("{}{}", start.to_uppercase(), rest))
}

#[derive(Deserialize, Debug)]
struct Config {
    branches: Branches,
    title: Title,
    template: Template,
    labels: Labels,
    commits: Commits,
    draft: bool,
    dry_run: bool,
}
#[derive(Deserialize, Debug)]
struct Branches {
    default: String,
    includes: HashMap<String, String>,
}
#[derive(Deserialize, Debug)]
struct Title {
    jira_prefixes: HashMap<String, String>,
    prefixes: HashMap<String, String>,
}
#[derive(Deserialize, Debug)]
struct Template {
    path: String,
}
#[derive(Deserialize, Debug)]
struct Labels {
    default: Vec<String>,
    includes: HashMap<String, Vec<String>>,
}
#[derive(Deserialize, Debug)]
struct Commits {
    prefixes: HashMap<String, String>,
}

fn get_config() -> Result<Config, ScriptErrors> {
    let exe_path = env::current_exe().map_err(|e| {
        ScriptErrors::ConfigError(format!("Failed to get current executable path: {}", e))
    })?;

    let exe_dir = exe_path.parent().ok_or(ScriptErrors::ConfigError(format!(
        "Failed to get parent directory of executable"
    )))?;
    let config_path = exe_dir.join("config.toml");

    let config = fs::read_to_string(config_path)
        .map_err(|e| ScriptErrors::ConfigError(format!("Failed to read config file: {}", e)))?;

    toml::from_str(&config).map_err(|e| ScriptErrors::ParseError(e.to_string()))
}

fn get_pr_title(branch_name: &str, config: &Config) -> String {
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

fn get_target_branch(branch_name: &String, config: &Config) -> String {
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

fn remove_ansi_codes(s: &str) -> String {
    // Regex to match ANSI escape codes
    let re = Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]")
        .unwrap_or_else(|e| panic!("Failed to compile regex to remove ansi codes: {}", e));
    re.replace_all(s, "").to_string()
}

use regex::Regex;
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

fn get_commit_body(config: &Config, target_branch: &String) -> Result<String, ScriptErrors> {
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

fn get_pr_labels(config: &Config, branch_name: &String) -> Vec<String> {
    let mut labels = config.labels.default.clone();

    for (key, value) in config.labels.includes.iter() {
        if branch_name.to_lowercase().contains(key) {
            labels.extend(value.clone());
        }
    }

    labels
}

fn main() -> Result<(), ScriptErrors> {
    let config = get_config()?;

    let output = std::process::Command::new("git")
        .args(&["branch", "--show-current"])
        .output()
        .map_err(|e| ScriptErrors::ConfigError(format!("Failed to execute git command: {}", e)))?;

    let branch_name = String::from_utf8_lossy(&output.stdout).to_lowercase();

    let pr_title = get_pr_title(&branch_name, &config);

    let target_branch = get_target_branch(&branch_name, &config);

    // Get a list of commits since the branch was created

    let commit_body = get_commit_body(&config, &target_branch)?;

    let pr_labels = get_pr_labels(&config, &branch_name).join(",");

    let draft = config.draft;

    // print whole command
    let mut args = vec![
        "pr",
        "create",
        "-a",
        "@me",
        "-t",
        &pr_title,
        "--body",
        &commit_body,
        "-B",
        target_branch.as_str(),
    ];

    if !pr_labels.is_empty() {
        args.push("-l");
        args.push(pr_labels.as_str());
    }

    if draft {
        args.push("-d");
    }

    let extra_args: Vec<String> = env::args().skip(1).collect();

    for arg in &extra_args {
        args.push(arg.as_str());
    }

    if config.dry_run {
        println!("Dry run, not creating PR");
        println!("PR created: {}", String::from_utf8_lossy(&output.stdout));
        println!("Title: {}", pr_title);
        println!("Target branch: {}", target_branch);
        println!("Commit body: \n{}", commit_body);
        println!("PR labels: {}", pr_labels);
        args.push("--dry-run");
    }
    let output = std::process::Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| {
            ScriptErrors::ConfigError(format!(
                "{} {}",
                "Github CLI not installed please go to https://cli.github.com and install it", e
            ))
        })?;

    if !output.status.success() {
        println!(
            "Error creating PR: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        println!("PR created: {}", String::from_utf8_lossy(&output.stdout));
        println!("Title: {}", pr_title);
        println!("Target branch: {}", target_branch);
    };
    return Ok(());
}
