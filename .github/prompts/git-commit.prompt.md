---
mode: agent
---

Your job is to recreate all commits on the current Git branch. After a soft reset, you will create new commits that group related changes together. Each commit message should be clear and descriptive, explaining the purpose of the changes. You will create as many commits as needed to group the changes together.

Always do these steps first.

1. Ensure all changes are staged and then commit them with a work-in-progress message
2. Pull the origin `main` branch
3. Rebase the current branch onto the `main` branch
4. Soft reset the current branch to the `main` branch
5. Restore all the staged files to their unstaged state

Read all the changed files, and for each set of related changes stage the files that belong to the current group of related changes:

- Do not use `git add .` or `git add -A` to stage all files at once
- For each commit, only stage the files that belong to the current group of related changes
- Stage only the files for the current set of changes
- Create a new commit for this group of related changes
- Always use conventional commit format for commit messages
- Keep the subject line under 72 characters
- Write a detailed body for the commit message, explaining the changes and their purpose
- Repeat for each group of related changes