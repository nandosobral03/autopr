#[cfg(test)]

mod test {

    use std::collections::HashMap;

    use crate::config::{Branches, Commits, Config, Labels, Template, Title};
    use crate::git::get_pr_title;

    fn test_config() -> Config {
        let mut jira_prefixes = HashMap::new();
        jira_prefixes.insert(
            "htp20".to_string(),
            "[HTP20-{ticket_number}] {ticket_name}".to_string(),
        );

        let mut prefixes = HashMap::new();
        prefixes.insert("hotfix".to_string(), "HOTFIX: ".to_string());

        Config {
            branches: Branches {
                default: "main".to_string(),
                includes: HashMap::new(),
            },
            title: Title {
                jira_prefixes,
                prefixes,
            },
            template: Template {
                path: ".github/PULL_REQUEST_TEMPLATE.md".to_string(),
            },
            labels: Labels {
                default: vec!["bug".to_string(), "enhancement".to_string()],
                includes: HashMap::new(),
            },
            commits: Commits {
                prefixes: HashMap::new(),
            },
            draft: false,
            dry_run: false,
        }
    }

    #[test]
    fn test_get_pr_title() {
        let config = test_config();
        let branch_name = "htp20-123-test-title-jira-prefix";
        let pr_title = get_pr_title(&branch_name, &config);
        assert_eq!(pr_title, "[HTP20-123] Test Title Jira Prefix");
    }

    #[test]
    fn test_get_pr_title_case_insensitive() {
        let config = test_config();
        let branch_name = "HTP20-123-TEST-TITLE-JIRA-PREFIX";
        let pr_title = get_pr_title(&branch_name, &config);
        assert_eq!(pr_title, "[HTP20-123] TEST TITLE JIRA PREFIX");
    }

    #[test]
    fn test_get_pr_title_without_prefix() {
        let config = test_config();
        let branch_name = "random-branch-name";
        let pr_title = get_pr_title(&branch_name, &config);
        assert_eq!(pr_title, "Random Branch Name");
    }

    #[test]
    fn test_get_pr_title_single_word() {
        let config = test_config();
        let branch_name = "singleword";
        let pr_title = get_pr_title(&branch_name, &config);
        assert_eq!(pr_title, "Singleword");
    }

    #[test]
    fn test_get_pr_title_empty_branch() {
        let config = test_config();
        let branch_name = "";
        let pr_title = get_pr_title(&branch_name, &config);
        assert_eq!(pr_title, "");
    }

    #[test]
    fn test_get_pr_title_with_prefix() {
        let config = test_config();
        let branch_name = "hotfix-test-title-jira-prefix";
        let pr_title = get_pr_title(&branch_name, &config);
        assert_eq!(pr_title, "HOTFIX: Test Title Jira Prefix");
    }
}
