---
description: Ship a feature with the dev-pipeline. You approve the plan, then develop, test and review loop automatically.
argument-hint: <feature description | owner/repo#123 | path/to/spec.md>
---

# /ship-feature

Shortcut for `/run-workflow feature <input>`.

1. If `$ARGUMENTS` is empty, ask the user what feature to ship and stop.
2. Read `.claude/commands/run-workflow.md`. If there's no project copy, read `~/.claude/commands/run-workflow.md`.
3. Follow its instructions exactly, as if it had been invoked with the arguments `feature $ARGUMENTS`.

**Plan before executing (mandatory):** the `feature` workflow starts with the Planner. Nothing in the project changes until the user explicitly approves the Planner's spec. If a scope change comes up later, the run goes back to planning for re-approval. The run commits only checkpoints on its own branch. Nothing is pushed until you say “land it”, and you merge the pull request yourself.
