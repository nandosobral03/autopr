#[cfg(test)]
mod test {
    use crate::{config::test_config, git::get_pr_title};

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
}
