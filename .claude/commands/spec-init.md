---
description: Bootstrap spec-driven development in an existing project. Drafts the constitution and a system overview from the repo for your approval.
argument-hint: [optional focus notes, e.g. "focus on services/api"]
---

# /spec-init

Shortcut for `/run-workflow spec-init <input>`.

1. If `$ARGUMENTS` is empty, use the input `Bootstrap specs for this project.`
2. Read `.claude/commands/run-workflow.md`. If there's no project copy, read `~/.claude/commands/run-workflow.md`.
3. Follow its instructions exactly, as if it had been invoked with the arguments `spec-init <input>`.

**Plan before executing (mandatory):** the extractor is read-only. Nothing is written to the project until you approve both drafts. If a constitution already exists, the drafts propose changes to it, and your hand edits are never overwritten without asking. Nothing is ever committed or pushed.
