---
mode: agent
---

Your job is to recreate all commits on the current Git branch. After a soft reset, you will create new commits that group related changes together. Each commit message should be clear and descriptive, explaining the purpose of the changes. You will create as many commits as needed to group the changes together.

Always do these steps first.

1. Ensure all changes are staged and then commit them with a work-in-progress message. There may be no unstaged changes, if not, then keep going.
2. Pull the origin `main` branch
3. Rebase the current branch onto the `main` branch
4. Soft reset the current branch to the `main` branch
5. Restore all the staged files to their unstaged state

Next, follow these steps:

1. Read all the changed files
2. Group them into logical sets of changes
3. Add each logical set of changes to the staging area
4. Commit the logical set of changes with an appropriate commit message
5. Repeat for each group of related changes

Here are some general rules:

- Do not use `git add .` or `git add -A` to stage all files at once
- Create a new commit for each group of logically related changes
- Always use conventional commit format for commit messages
- Keep the subject line under 72 characters
- Write a detailed body for the commit message, explaining the changes and their purpose
