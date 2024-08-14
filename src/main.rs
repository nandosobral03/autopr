use config::get_config;
use errors::ScriptErrors;
use git::{get_commit_body, get_pr_labels, get_pr_title, get_target_branch};
use std::env;
mod config;
mod errors;
mod git;
mod utils;

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
