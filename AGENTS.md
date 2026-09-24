# AGENTS.md

Wade is an animated desk companion for the M5Stack CoreS3 Lite, written in Rust and developed on desktop first.

## Docs

Start at [docs/README.md](docs/README.md). The design docs are the source of truth for architecture, behavior, testing, and the roadmap; read the relevant one before changing an area, and update it when a change makes it wrong.

## Environment

- The toolchain is pinned in `rust-toolchain.toml`; `rustup` installs it on first use.
- `wade-desktop` needs SDL2 (`brew install sdl2`). On Apple Silicon, linking needs the Homebrew library path:

  ```sh
  export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$(brew --prefix)/lib"
  ```

- Enable the pre-commit hook once per clone with `git config core.hooksPath .githooks`. Superset workspaces do this in setup.

## Skills

- `verify`: run the local checks and tests.
- `create-pr`: open a PR with the right title and template.

## Agent improvements

Skills, rules, and other agent directives follow the [AGENTS.md](https://agents.md) and [Agent Skills](https://agentskills.io) conventions and live in this file or under `.agents/`. Never put them in an agent-specific location such as `.claude/`.

Claude Code reads this file but finds skills only under `.claude/skills/`. To add a skill, write it at `.agents/skills/<name>/SKILL.md`, then symlink it for Claude:

```sh
ln -s ../../.agents/skills/<name> .claude/skills/<name>
```

## Commits

Split work into logical commits, each a coherent change that makes sense on its own; keep related changes together and unrelated ones apart. Start every message with a conventional-commit prefix (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `ci:`, `perf:`), then a short lowercase imperative summary: `fix: stop blink from skipping frames`.

## Wade

Wade is helpful, smart, and quick-witted, loyal to the person at the desk, and a little too proud of himself; sometimes that pride backfires. He speaks only through expression, gaze, and small actions. Keep changes to his behavior and look true to that persona; [docs/character.md](docs/character.md) has the details.

## Code comments

A comment must make sense to any future reader of the code and say something the code doesn't. Don't write comments that only matter in the current working context or review, such as what changed, what was tried, or replies to reviewers. When in doubt, leave it out.

## Voice

Less is more. Keep prose, docs, commit messages, and PR descriptions tight and to the point.

In docs, don't reproduce the project's layout or directory structure. A high-level overview of the general structure is fine when it helps; file names, line numbers, and other details that churn go stale.
