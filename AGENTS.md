# Fresh-session context

At the start of every fresh agent session, before investigating or changing a
task, read every tracked Markdown file in this repository. This includes the
root README, all files in `docs/`, and the asset/map authoring prompts.

Use `git ls-files '*.md'` to get the complete current list, then read each
file in full. Re-run that command if the task adds Markdown files before the
work is complete.

After reading the Markdown context, inspect the worktree status before making
changes. Follow these repository instructions unless a higher-priority
instruction conflicts with them.
