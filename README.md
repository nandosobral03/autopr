# Project Title

A tool for automating the creation of templated pull requests based on branch names and commit differences with the target branch. It is designed to work seamlessly with [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) and integrates with [Jira](http://jira.com/)'s branch naming conventions.

## Installation

1. Download a release binary from the [releases page](https://github.com/fernandosobral/ghpr-script/releases).
2. Create or obtain a `config.toml` file and place it in the **same directory as the binary**. For details on configuring this file, refer to the [Config](#config) section.
3. Navigate to a git repository and execute the binary.

**Note:** Ensure you have [GitHub CLI](https://cli.github.com/) installed and authenticated, as it is required to create pull requests.

## Config

The `config.toml` file should be located in the **same directory as the binary**. Below is an example configuration:

```toml
draft = false # For this to work draft pr must be enabled on the repo
dry_run = true # If true, the script will not create a PR, but print the command that would be executed

# Target branch configuration
[branches] # If no other configuration is found, the default branch will be used
default = "develop"

[branches.includes] # If the branch name includes the word "hotfix", the target branch will be "main"
# You can add other rules with the same logic, for example:
# If the branch name includes the word "feature", the target branch will be "develop"
hotfix = "main"

[title] # By default the PR title with be the branch name, changed from kebab case to title case e.g.
# branch-1234-my-ticket-name -> Branch 1234 My Ticket Name.
# If using jira branch naming convention, the {ticket_number} and {ticket_name}
#will be replaced with the ticket number and name

[title.jira_prefixes] # This is the project prefix on jira, {ticket_number} and {ticket_name} will
# be replaced with the ticket number and name if using the jira branch naming convention
htp20 = "[HTP20-{ticket_number}] {ticket_name}" # e.g. "HTP20-1234 My ticket name"

[title.prefixes] # This will replace the prefix used with a given prefix
hotfix = "Hotfix:"


[template] # Absolute path to the template file
path = "/Users/[username]/path/to/template.md"

[labels]
default = ["draft"] # All branches will be assigned the default labels

[labels.includes] # If the branch name includes the word "hotfix", the labels will also
# include "hotfix"
hotfix = ["hotfix"]


[commits] # How your commits will be formatted on the PR
# If the commit message starts with "feat:", the PR will be prefixed with the text ""
# If the commit message starts with "fix:", the PR will be assigned the label "Fix: "
[commits.prefixes] # Prefixes are assumed to end on : or (scope): as per conventional commits
feat = ""
fix = "Fix:"
chore = "Chore:"
```

Refer to the example `template.md` file in the repository for a template format.

Usage
Authenticate with GitHub CLI using the gh command to allow the script to create pull requests on your behalf.

Run the binary in a git repository to create a pull request. The script will generate the PR based on branch name, commit differences, and the provided configuration.

For convenience, create an alias or script to simplify running the binary. Here’s an example:

#### Create a script ~/scripts/ghpr.sh:

```sh
#!/bin/bash
~/scripts/path-to-binary
```

#### Add the script directory to your PATH:

Append the following to your ~/.bashrc or ~/.zshrc:

```sh
export PATH="$PATH:$HOME/scripts"
```
