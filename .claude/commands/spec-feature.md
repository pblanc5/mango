---
description: Spec-driven feature with the dev-pipeline. You approve the critiqued requirements, then the design and tasks, which are published to the specs folder, before develop, test and review.
argument-hint: <feature description | owner/repo#123 | path/to/specs/<id>/requirements.md>
---

# /spec-feature

Shortcut for `/run-workflow spec-feature <input>`.

1. If `$ARGUMENTS` is empty, ask the user what feature to specify and stop.
2. Read `.claude/commands/run-workflow.md`. If there's no project copy, read `~/.claude/commands/run-workflow.md`.
3. Follow its instructions exactly, as if it had been invoked with the arguments `spec-feature $ARGUMENTS`.

**Plan before executing (mandatory):** only read-only personas, the spec writer and spec critic, run before your first approval. You approve the requirements, then the design and tasks, before any code changes. Approved documents are published to the specs folder. The run commits only checkpoints on its own branch. Nothing is pushed until you say “land it”, and you merge the pull request yourself. Pointing at an existing spec amends it and keeps its IDs.
