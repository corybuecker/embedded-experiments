---
mode: agent
---

The goal is to recreate all commits on the current Git branch into one or more commits that group related changes together. Using a soft reset, you will create new commits that group related changes together. The commit messages should be clear and descriptive, explaining the purpose of the changes.

Initial steps:

- Ensure all changes are staged and then commit them with a work-in-progress message.
- Pull the origin `main` branch first.
- Rebase the current branch onto the `main` branch.
- Soft reset the current branch to the `main` branch.

Recreate commits:

- Always use conventional commit format for commit messages.
- Keep the subject line under 72 characters.
- Avoid git commands that require user input, such as `git commit --amend` or `git rebase -i`.
