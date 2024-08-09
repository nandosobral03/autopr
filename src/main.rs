use core::panic;
use std::{collections::HashMap, env, fs, path::Path};

use serde::Deserialize;

fn capitalize_word(s: &str) -> String {
    let start = s.chars().next().unwrap();
    let rest = s.chars().skip(1).collect::<String>();
    format!("{}{}", start.to_uppercase(), rest)
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

fn get_config() -> Config {
    let config =
        fs::read_to_string(Path::new("./config.toml")).expect("Could not read config.toml");
    toml::from_str(config.as_str()).expect("Could not parse config.toml")
}

fn get_pr_title(branch_name: &String, config: &Config) -> String {
    match branch_name.split("-").next() {
        Some(branch_start) => match config.title.jira_prefixes.get(branch_start) {
            Some(pr_title) => match branch_name.split("-").nth(1) {
                Some(ticket_number) => pr_title.replace("{ticket_number}", ticket_number).replace(
                    "{ticket_name}",
                    branch_name
                        .split("-")
                        .skip(2)
                        .collect::<Vec<&str>>()
                        .join(" ")
                        .as_str(),
                ),
                None => branch_name.clone(),
            },
            None => match config.title.prefixes.get(branch_start) {
                Some(pr_title) => branch_name
                    .replacen(branch_start, pr_title, 1)
                    .replace("-", " ")
                    .to_string(),

                None => branch_name.clone(),
            },
        },
        None => branch_name
            .split("-")
            .map(|word| capitalize_word(word))
            .collect::<Vec<String>>()
            .join(" "),
    }
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
    let re = Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").unwrap();
    re.replace_all(s, "").to_string()
}

use regex::Regex;
fn normalize_commits(commits: &str, prefixes: &[String]) -> Vec<String> {
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
    let re = match Regex::new(&pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("Failed to compile regex: {}", e);
            return Vec::new();
        }
    };

    let mut replaced_commits: Vec<String> = commits
        .lines()
        .rev()
        .filter(|line| !line.is_empty())
        .filter(|line| re.is_match(line))
        .map(|line| {
            let matched_prefix = re.find(line).unwrap().as_str();
            // Get the real prefix instead of the regex capture group (removing the scope)
            let mut clean_prefix = String::new();
            for c in prefixes.iter() {
                if matched_prefix.contains(c) {
                    clean_prefix = c.to_string();
                    break;
                }
            }
            // The message is the rest of the line after the prefix, it's not captured
            let message = line.split(matched_prefix).nth(1).unwrap().trim();
            format!("{}: {}", clean_prefix, message)
        })
        .map(|line| remove_ansi_codes(&line).to_string())
        .collect();

    // sort by first their prefix
    replaced_commits.sort_by(|a, b| {
        let a_prefix = a.split(": ").next().unwrap();
        let b_prefix = b.split(": ").next().unwrap();
        a_prefix.cmp(b_prefix)
    });

    replaced_commits
}

fn get_commit_body(config: &Config, target_branch: &String) -> String {
    let mut commit_body = String::from_utf8(fs::read(config.template.path.clone()).unwrap())
        .map_err(|_| "Could not read template file, check the path in config.toml")
        .unwrap();

    let output = std::process::Command::new("git")
        .args(&["log", "--oneline", &format!("{}..HEAD", target_branch)])
        .output()
        .expect("Failed to get commit list");

    let commits = String::from_utf8_lossy(&output.stdout);

    let relevant_commit_prefixes = config
        .commits
        .prefixes
        .iter()
        .map(|(key, _value)| key.clone())
        .collect::<Vec<String>>();

    let commits_as_body = normalize_commits(&commits, &relevant_commit_prefixes)
        .iter()
        // Map with the config
        .map(|line| {
            let matching_prefix = relevant_commit_prefixes
                .iter()
                .find(|prefix| line.contains(format!("{}:", prefix).as_str()))
                .expect("Could not find matching prefix in commit message");
            let prefix_config = config.commits.prefixes.get(matching_prefix).unwrap();

            let replaced = line
                .clone()
                .replacen(format!("{}:", matching_prefix).as_str(), prefix_config, 1)
                .trim()
                .to_string();
            //Turn the first letter of the message to uppercase, the rest stays the same

            let first_letter = &replaced.chars().next().unwrap();
            let capitalized = format!(
                "{}{}",
                first_letter.to_uppercase(),
                replaced.chars().skip(1).collect::<String>()
            )
            .to_string();

            return format!("- {}", capitalized).to_string();
        })
        .collect::<Vec<String>>()
        .join("\n");

    if !commit_body.contains("{LIST_COMMITS}") {
        panic!("Could not find LIST_COMMITS in commit body");
    }

    commit_body = commit_body.replace("{LIST_COMMITS}", &commits_as_body);
    commit_body
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

fn main() {
    let config = get_config();

    let output = std::process::Command::new("git")
        .args(&["branch", "--show-current"])
        .output()
        .expect("failed to execute process");

    let branch_name = String::from_utf8_lossy(&output.stdout).to_lowercase();

    let pr_title = get_pr_title(&branch_name, &config);

    let target_branch = get_target_branch(&branch_name, &config);

    // Get a list of commits since the branch was created

    let commit_body = get_commit_body(&config, &target_branch);

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

    let extra_args: Vec<String> = env::args().skip(1).collect();
    for arg in &extra_args {
        args.push(arg.as_str());
    }

    if !pr_labels.is_empty() {
        args.push("-l");
        args.push(pr_labels.as_str());
    }

    if draft {
        args.push("-D");
    }

    if config.dry_run {
        println!("Dry run, not creating PR");
        println!("PR created: {}", String::from_utf8_lossy(&output.stdout));
        println!("Title: {}", pr_title);
        println!("Target branch: {}", target_branch);
        println!("Commit body: \n{}", commit_body);
        println!("PR labels: {}", pr_labels);
        // println!("Would execute command: gh {}", args.join(" "));
    } else {
        let output = std::process::Command::new("gh")
            .args(args)
            .output()
            .expect("Failed to create PR");

        if !output.status.success() {
            println!(
                "Error creating PR: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            println!("PR created: {}", String::from_utf8_lossy(&output.stdout));
            println!("Title: {}", pr_title);
            println!("Target branch: {}", target_branch);
        }
    }
}
