---
name: create-pr
description: Open a GitHub pull request for wade with a conventional-commit title and the repo's PR template. Use when asked to open, create, or submit a PR.
---

# Create PR

1. Run the `verify` skill first. Don't open a PR with failing checks unless the user asks to.
2. Work on a branch, never `main`. Push it with `git push -u origin HEAD`.
3. Open the PR with `gh pr create --title "<title>" --body-file <file>`.

## Title

Starts with a conventional-commit prefix: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `ci:`, or `perf:`. Lowercase and imperative after the prefix, with no trailing period: `feat: add blink animation`.

## Body

Fill every section of `.github/pull_request_template.md` and drop its HTML comments.

- **Reason for change**: why the change exists, in a sentence or two. Add `Closes #NN` when there is an issue.
- **Scope**: high level only. Name the crates or areas affected and the blast radius. Don't list files or restate the diff; reviewers have it.
- **Verification**: the checks that ran and their results, plus any runtime verification such as running the simulator.

Keep it short. A reader should get the point in under a minute.
