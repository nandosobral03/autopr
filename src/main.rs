fn capitalize_word(s: &str) -> String {
    let start = s.chars().next().unwrap();
    let rest = s.chars().skip(1).collect::<String>();
    format!("{}{}", start.to_uppercase(), rest)
}

fn main() {
    let output = std::process::Command::new("git")
        .args(&["branch", "--show-current"])
        .output()
        .expect("failed to execute process");

    println!("Branch name: {}", String::from_utf8_lossy(&output.stdout));
    let branch_name = String::from_utf8_lossy(&output.stdout).to_lowercase();

    // check the start of the branch name
    let new_name: String = match branch_name.split("-").next() {
        Some(branch_start) => match branch_start {
            "htp20" => {
                let ticket_number = branch_name.split("-").nth(1).unwrap();
                format!(
                    "[HTP20-{}] {}",
                    ticket_number,
                    branch_name
                        .split("-")
                        .skip(2)
                        .map(|word| capitalize_word(word))
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            }
            _ => branch_name
                .split("-")
                .map(|word| capitalize_word(word))
                .collect::<Vec<String>>()
                .join(" "),
        },
        None => {
            // change kebab case capitalizing each word
            branch_name
                .split("-")
                .map(|word| capitalize_word(word))
                .collect::<Vec<String>>()
                .join(" ")
        }
    };

    let target_branch = if branch_name.to_lowercase().contains("hotfix") {
        "main"
    } else {
        "develop"
    };

    // Get a list of commits since the branch was created
    let output = std::process::Command::new("git")
        .args(&["log", "--oneline", &format!("{}..HEAD", target_branch)])
        .output()
        .expect("Failed to get commit list");

    let commits = String::from_utf8_lossy(&output.stdout);

    let features = commits
        .lines()
        .rev()
        .map(|line| line.chars().skip(18).collect::<String>())
        .filter(|line| line.contains("feat:"))
        .map(|line| {
            let parts = line.split("feat:").collect::<Vec<&str>>();
            let line = parts[1].trim();
            format!("- {}", capitalize_word(line))
        })
        .collect::<Vec<String>>()
        .join("\n");

    let fixes = commits
        .lines()
        .rev()
        .map(|line| line.chars().skip(18).collect::<String>())
        .filter(|line| line.contains("fix:"))
        .map(|line| {
            let parts = line.split("fix:").collect::<Vec<&str>>();
            let line = parts[1].trim();
            capitalize_word(line)
        })
        .map(|line| format!("- Fix {}", line))
        .collect::<Vec<String>>()
        .join("\n");

    let mut commit_body = String::from("## Describe your changes\n\n");
    commit_body.push_str(&features);
    commit_body.push_str("\n\n");
    commit_body.push_str(&fixes);
    commit_body.push_str("\n\n");
    commit_body.push_str("## Screenshots (if appropriate)\n\n");
    commit_body.push_str("## Checklist\n\n");
    commit_body.push_str("- [ ] Moved the ticket to Code Review\n");
    commit_body.push_str("- [ ] Uploaded screenshots (if appropriate)\n");
    commit_body.push_str("- [x] Run linter rules\n");
    commit_body.push_str("- [x] Run tests (and fix them if needed)\n");
    commit_body.push_str("\n\n");
    commit_body.push_str("## Picture of a cute animal\n");

    let output = std::process::Command::new("gh")
        .args(&[
            "pr",
            "create",
            "-a",
            "@me",
            "-t",
            &new_name,
            "--body",
            &commit_body,
            "-B",
            target_branch,
            "-l",
            // Always add draft, if it's a hotfix also add the hotfix label comma separated
            if target_branch == "main" {
                "draft,hotfix"
            } else {
                "draft"
            },
        ])
        .output()
        .expect("Failed to create PR");

    if !output.status.success() {
        println!(
            "Error creating PR: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        println!("New name: {}", new_name);
        println!("PR created: {}", String::from_utf8_lossy(&output.stdout));
    }
}
