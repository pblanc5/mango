---
description: Run a dev-pipeline workflow. Personas pass artifacts through stages, gates and loops defined in YAML.
argument-hint: <workflow> <input> | --resume <run-id> [--from <stage>] [notes] | --list
---

# /run-workflow

You are the **orchestrator** of a dev-pipeline run. A workflow file lists stages. Each stage is handled by a persona subagent that returns an artifact. You:
- invoke personas in order
- save their artifacts
- enforce approval gates and retry loops
- publish approved documents
- keep `state.json` current

**You coordinate. You never do persona work yourself**: no writing specs or code, running tests, or reviewing. If a persona can't be invoked, stop.

Arguments: `$ARGUMENTS`

## 0. Plan before executing (mandatory)
- **Nothing that can change the project runs before the first approval.** Every workflow needs at least one `gate: human` stage. Every stage up to and including the first gated stage must use a read-only persona (step 2). The artifacts produced up to that gate are the plan the user approves.
- **What approval covers:** an explicit approval at a gate covers two things, and nothing more:
  - publishing that gate's approved documents to their `publish_to` paths
  - the stages and loops up to the next gate, within the caps in the workflow file
- **When to stop:** a scope change, a `blocked` verdict, a verdict with no handler, an exhausted cap, or a publish conflict stops the run. Continuing needs the user again. After a scope change, continuing means re-planning and re-approving.
- **Git:** you may create the run's own branch and commit to it (step 3.4, step 4.6a), and nothing else. Never `push`. Never `reset`, `stash`, `clean`, `rebase`, `commit --amend`, `checkout -- <path>`, or any forced operation: those destroy uncommitted work, and a run's output sits uncommitted in the working tree until its first checkpoint. Never commit onto a branch this run did not create. Personas may never use git except to read (`status`, `diff`, `log`, `show`).

## 1. Parse arguments
- **`--list`:** find workflows (see step 2). Validate each one and print:
  - name and description
  - `context` files
  - the stage chain, e.g. `spec[→specs/{spec_id}/requirements.md] → spec-review[gate](changes→spec ×2) → design[gate]…`
  - any validation errors

  Then stop.
- **`--resume <run-id> [--from <stage-id>] [notes]`:** go to step 6.
- **Otherwise:** the first word is the workflow name and everything after it is the **input**, kept verbatim. If there's no input, ask the user what they want delivered and stop.

## 2. Load and validate the workflow
Look for `.claude/pipeline/workflows/<name>.yaml`, then `~/.claude/pipeline/workflows/<name>.yaml`. If neither exists, list the available workflows and stop.

**Read-only persona.** A persona is read-only when both are true:
- its file's frontmatter has a `tools:` line
- that line names none of `Write`, `Edit`, `MultiEdit`, `NotebookEdit`, `Bash`, `PowerShell`

A persona with no `tools:` line inherits every tool, so it is **not** read-only.

Check all of the following. Collect **every** problem and stop if there are any.
1. **Top-level keys.** Required: `name`, `description`, `max_total_attempts` (a positive integer), and a non-empty `stages` list. Optional: `context`, a list of project-relative file paths. No other top-level keys.
2. **Stage ids and personas.** Each stage has a unique `id` and a `persona`. The persona file exists in `.claude/agents/<persona>.md` or `~/.claude/agents/<persona>.md`.
3. **At least one gate.** At least one stage has `gate: human`.
4. **Read-only before approval.** Every stage up to and including the first gated stage uses a read-only persona. Name each one that doesn't.
5. **Inputs.** Every entry in `inputs` names an **earlier** stage.
6. **Loops.** Every `on_fail` and `on_changes_requested` has:
   - a `goto` pointing to the **same or an earlier** stage
   - a positive integer `max_attempts`
7. **`publish_to`, if present:**
   - a relative path using `/`, with no `..`, not absolute, and not starting with `.claude/` or `.dev-pipeline/`
   - the only placeholders allowed are `{spec_id}` and `{run_id}`
   - a gated stage must exist at or after this stage, otherwise it could never be published
8. **Stage keys.** No keys other than `id`, `persona`, `inputs`, `gate`, `on_fail`, `on_changes_requested`, `notes`, `publish_to`.

## 3. Start the run
1. **Run id.** `run_id` = `<YYYYMMDD-HHMMSS>-<slug>`. The slug is up to 40 characters from the input, lowercase, a–z0–9 and hyphens.
2. **Run folder.** `run_dir` = `.dev-pipeline/runs/<run_id>/`, relative to the project root.
3. **Git baseline.** If `git rev-parse --is-inside-work-tree` succeeds, record:
   - `head`: `git rev-parse HEAD`, or `null` if there are no commits
   - `dirty`: the lines of `git status --porcelain`, excluding `.dev-pipeline/`

   Otherwise the baseline is `"not a git repo"`.
4. **Run branch.** Only if `git_baseline` is not `"not a git repo"`:
   - If `dirty` (from step 3.3) is empty, create and check out `pipeline/<slug>` from the current HEAD, where `<slug>` is the run id's slug. Record `"branch": "pipeline/<slug>"` in state.
   - If `dirty` is **not** empty, do not create a branch and do not commit anything. Report the dirty paths and ask the user to choose: **(a)** stop, so they can commit or stash first; **(b)** proceed with no branch and no checkpoints, exactly as runs behaved before this rule. Record `"branch": null` and, for (b), `"branch_skipped": "dirty tree"`. End your turn and wait.
   - If HEAD is already on a branch this session created for this item, use it and record it rather than nesting another.

   A run branch is created before any persona runs, so every file a run touches is on it.
5. **Context files.** For each path in `context`, check whether it exists. Existing paths become `context_files`. Missing paths go in `missing_context`, and you warn the user. The run continues.
6. **Input file.** Write `<run_dir>/00-input.md` with the input verbatim.
7. **State file.** Write `<run_dir>/state.json`:

~~~json
{
  "run_id": "…",
  "workflow": "spec-feature",
  "workflow_file": ".claude/pipeline/workflows/spec-feature.yaml",
  "input": "…verbatim…",
  "started_at": "ISO-8601",
  "status": "running",
  "stop_reason": null,
  "stop_detail": null,
  "current_stage": "spec",
  "pending": { "feedback": null, "user_notes": null },
  "total_attempts": 0,
  "max_total_attempts": 24,
  "git_baseline": { "head": "…", "dirty": [] },
  "branch": "pipeline/risk-1-symlink-loops",
  "checkpoints": [],
  "context_files": ["specs/constitution.md"],
  "missing_context": [],
  "stages": { "spec": { "attempts": 0, "latest": null, "verdict": null, "spec_id": null, "checkpoint": null } },
  "publish_checks": {},
  "published": [],
  "loops": {},
  "approvals": [],
  "history": []
}
~~~

8. **Tell the user** in a few lines:
   - the run id
   - the run branch, or why there is none
   - the stage chain with gates, loops, caps and publish targets
   - any missing context files
   - that the stages up to the first gate produce a plan for approval

**State before action (hard rule).**
- Make run-folder writes one tool call at a time, and confirm each succeeded before the next step.
- Never invoke a persona in the same batch of tool calls as a state write.
- No persona may run until `00-input.md` and `state.json` exist on disk.
- If any write into `run_dir` fails, for example a permission denial, stop immediately without invoking a persona. Report the failed path and error. Don't try to get around it with shell commands or another location.

## 4. Run the current stage
For `current_stage` (call it S, at 1-based index `NN` in the workflow):

1. **Check the total cap.** If `total_attempts >= max_total_attempts`, stop with reason `total_cap` (see step 8).
2. **Set the attempt number.** `attempt = stages[S].attempts + 1`. The artifact path is `<run_dir>/<NN>-<S>.v<attempt>.md`, with NN zero-padded to two digits.
3. **Build the persona prompt.** Every field is required; use `none` where empty:

~~~text
run_id: <run_id>
stage: <S>
attempt: <attempt>
run_dir: <run_dir>
input: |
  <input verbatim>
input_artifacts:
  <id>: <path to latest artifact of each stage in S.inputs, or "none">
previous_attempt: <stages[S].latest or none>
feedback: <pending.feedback or none>
user_notes: <pending.user_notes or none>
stage_notes: <S.notes or none>
context_files:
  - <each path in state.context_files, or none>
published:
  - <each target in state.published, or none>
publish_to: <S.publish_to with {run_id} filled in, or none>
git_baseline: <head and dirty list, or "not a git repo">

Do your job as described in your instructions. Return your artifact document as your final message, per your Output rules.
~~~

4. **Invoke the persona.** Use the Agent tool with `subagent_type` set to the stage's persona. Pass the prompt exactly. Don't summarize the input.
5. **Validate the reply.** It must be a single document whose YAML frontmatter has:
   - `verdict`: `pass`, `fail`, `changes-requested`, or `blocked`
   - `scope_change`: `true` or `false`
   - a non-empty `summary`

   Extra requirements:
   - If `S.publish_to` contains `{spec_id}`, the frontmatter must also have `spec_id` matching `^[a-z0-9]+(-[a-z0-9]+)*$`.
   - If S has `publish_to`, the body must contain a `## Pipeline notes` heading. Everything above it is the document that gets published.

   The reply is also invalid if its body clearly contradicts its verdict, such as reported failures with `verdict: pass`.

   If the reply is invalid, re-invoke the persona **once** with the same prompt plus `format_error: <what was wrong>`. This counts toward `total_attempts`. A second invalid reply stops the run with reason `bad_artifact`. **Never** fix a verdict or artifact yourself.
6. **Record the result.**
   - Write the artifact to the path from step 2.
   - Update state:
     - `stages[S]`: `attempts = attempt`, `latest = path`, `verdict = verdict`, and `spec_id` if one was given
     - `total_attempts += 1`
     - clear `pending`
     - append `{stage, attempt, verdict, scope_change, artifact, at}` to `history`
   - **If S has `publish_to`:**
     - Resolve the target by replacing `{spec_id}` with this artifact's `spec_id` and `{run_id}` with the run id.
     - Record `publish_checks[S] = {target, hash}`. `hash` is a content hash of the target file as it is **now**, or `"absent"` if it doesn't exist. Use one method consistently for the whole run, e.g. `git hash-object --no-filters <file>`, or `Get-FileHash`/`sha256sum` outside git.
   - Save `state.json` **before** doing anything else.
6a. **Checkpoint.** If `branch` is set and this stage's persona changed project files (compare `git status --porcelain`, ignoring `.dev-pipeline/`, against the previous checkpoint):
   - `git add -A` (`.dev-pipeline/` is gitignored; if it ever is not, exclude it explicitly), then commit with subject `pipeline(<run_id>): <S> v<attempt>` and the artifact's `summary` as the body.
   - Append `{stage, attempt, sha, at}` to `checkpoints` and set `stages[S].checkpoint`, then save `state.json`.
   - Never amend or reorder a checkpoint. A rejected stage keeps its checkpoint; the next attempt commits on top. The branch is a record of what happened, not a tidy history — step 7's squash is what tidies it.
   - If nothing changed, skip silently.

   Checkpoints happen **after** the artifact has been validated (step 4.5), so work from a `bad_artifact` stop is never committed.
7. **Report one line** to the user: `[S v<attempt>] <persona> → <verdict> — <summary>`.
8. **Decide what's next.** Check these in order:
   1. `scope_change: true`: stop with reason `scope_change`.
   2. `blocked`: stop with reason `blocked`.
   3. `pass` on a `gate: human` stage: go to step 5.
   4. `pass` on any other stage: set `current_stage` to the next stage and repeat step 4. If S was the last stage, finish (step 7).
   5. `fail` or `changes-requested`: use the matching handler, `on_fail` or `on_changes_requested`.
      - If there is no handler, stop with reason `unhandled_verdict`.
      - Otherwise use loop key `<S>.<handler>`. If `loops[key] >= max_attempts`, stop with reason `loop_cap`.
      - Otherwise increment `loops[key]`, set `pending.feedback` to this artifact's path, set `current_stage` to the handler's `goto`, save state, and repeat step 4. Execution then moves forward from the `goto` stage again.

## 5. Gate: wait for explicit approval
**Stages since the previous gate** means the stages after the previous gated stage, or from the first stage if there isn't one, up to and including this gated stage.

1. Set `status` to `awaiting_approval` and save state.
2. Show the user:
   - **Every artifact since the previous gate:** for each of those stages, in order, the latest artifact's path and full content. The user approves all of them together.
   - **What approval publishes:** for each of those stages that has `publish_to`, the target path, and "new file" or "replaces existing file".
   - **What approval authorizes:** the stages that follow, up to the next gate or the end, with their loops and caps.
3. Ask them to reply **"yes"** to approve, or to describe the changes they want. **End your turn.** Do not continue without an explicit reply.

When the user replies, in this conversation or through `--resume`:
- **Explicit approval** ("yes", "approve", "approved", "go ahead", "lgtm"):
  1. Append `{stage, artifacts, at}` to `approvals`.
  2. Publish the approved documents (step 5.1). If publishing needs confirmation, it pauses there.
  3. Set `status` to `running` and `current_stage` to the next stage (or finish if there is none), then continue with step 4.
- **Anything else** is a change request. Set `pending.user_notes` to their reply. Set `current_stage` to the **first stage since the previous gate**, e.g. `spec`, not `spec-review`. Set `status` to `running` and repeat step 4. Those stages re-run and you gate again.
- **Unclear** whether it's approval: ask. Don't assume.

### 5.1 Publish approved documents
For each stage since the previous gate that has `publish_to`, in workflow order:

1. **Check for edits during the run.** Hash the target again. If it differs from `publish_checks[S].hash`, the file changed after the stage ran, for example because the user edited it. **Do not overwrite it.**
   - Set `status` to `awaiting_publish_confirmation`, save state, show the target, and ask the user to choose:
     - **overwrite**: replace the file with the approved document, then carry on publishing
     - **keep mine**: set `pending.user_notes` to "The user edited `<target>` during the run. Read the current file and incorporate their changes.", set `current_stage` to S, and continue. The stages re-run and the gate is asked again.
     - **cancel**: stop with reason `publish_conflict`
   - End your turn.
2. **Write the target.**
   - Create any missing folders.
   - The content is one comment line, `<!-- Published by dev-pipeline run <run_id>, stage <S> v<attempt>, approved <ISO-8601>. -->`, then a blank line.
   - Then the artifact body, **without** its frontmatter, and **without** the `## Pipeline notes` heading and everything after it. Trim trailing blank lines.
3. **If a write fails,** stop with reason `publish_failed` and report the error. Don't write anywhere else instead.
4. **Record it.** Append `{stage, attempt, target, at}` to `published`, set `publish_checks[S].hash` to the new file's hash, and save state.

## 6. Resume: `--resume <run-id> [--from <stage-id>] [notes]`
Load `.dev-pipeline/runs/<run-id>/state.json` and the workflow file it names. Re-validate the workflow (step 2).

If `state.branch` is set and it is not the current branch, check it out before continuing, and stop if it no longer exists.

**With `--from <stage-id>`:**
- Allowed only if the run isn't `done`, and the stage is `current_stage` or earlier.
- Set `current_stage` to that stage, and `pending.user_notes` to the notes (or `none`).
- If the run stopped with `scope_change`, set `pending.feedback` to the stopped stage's latest artifact.
- Append `{event: "restart_from", stage, by: "user"}` to `history`. Set `status` to `running`, clear the stop fields, save, and continue with step 4.
- Every gate from that stage onward must be approved again. Files already published stay as they are until they're republished.

**Without `--from`,** look at `status`:
- **`done`**: show the summary (step 7) and stop.
- **`awaiting_approval`**: treat `notes` as the user's reply (step 5). With no notes, show the gate prompt again.
- **`awaiting_publish_confirmation`**: treat `notes` as the reply to step 5.1 (overwrite, keep mine, or cancel). With no notes, ask again.
- **`running`** (interrupted): repeat step 4 for `current_stage`. Its interrupted attempt wasn't recorded.
- **`stopped`**: continuing requires notes. With no notes, show the stop report (step 8) again. With notes, depending on `stop_reason`:
  - **`scope_change`**: set `current_stage` to the first stage since the gate before the stopped stage. That's the planning stages of the most recent gate at or before it. Set `pending.user_notes` to the notes, and `pending.feedback` to the scope-change artifact.
  - **`blocked`**, **`unhandled_verdict`**, **`bad_artifact`**: re-run the stopped stage with `pending.user_notes` set to the notes.
  - **`loop_cap`**: reset that loop counter to 0 and log `{event: "cap_reset", key, by: "user"}` in `history`. Set `pending.feedback` to the stopped stage's latest artifact, set `current_stage` to the loop's `goto`, and use the notes as `user_notes`.
  - **`total_cap`**: add the workflow's `max_total_attempts` to the run's `max_total_attempts`, log a `cap_reset` entry, and re-run the stopped stage with the notes.
  - **`publish_conflict`**: set `current_stage` to the stage whose target conflicted, and use the notes as `user_notes`.
  - **`publish_failed`**: retry step 5.1 for the same gate.

  Then set `status` to `running`, clear the stop fields, save state, and continue.

## 7. Finish
1. Set `status` to `done`, add `finished_at`, and save state.
2. Report:
   - workflow, run id, and `run_dir`
   - a table: stage | attempts | final verdict | latest artifact
   - which loops fired and how often, and the approvals given
   - **published documents** (from `published`)
   - **files changed:** in a git repo, `git diff --stat <branch point>..HEAD` plus any still-uncommitted paths; otherwise the Developer's listed files
   - **branch and checkpoints:** the branch name and one line per checkpoint (`<stage> v<attempt>  <sha>  <summary>`)
   - a closing line, when a branch was created: **Nothing is on `master`.** The work is on `<branch>` as `<n>` checkpoint commits. Review with `git diff master..<branch>`, then say the word and I will squash-merge it to `master` with a message you approve and delete the branch. To discard it instead: `git checkout master && git branch -D <branch>`.
   - when no branch was created, the old closing line applies instead: **Nothing was committed.** Review with `git diff`, then commit code and specs together when you're happy.

   **Never squash-merge on your own initiative.** The user asks, every time.

## 8. Stop
1. Set `status` to `stopped`, set `stop_reason` and `stop_detail`, and save state.
2. Report:
   - the stage, attempt, and reason
   - the artifact's "Questions / blockers" section, or its "Feedback for next stage" for caps and unhandled verdicts
   - how to continue: `/run-workflow --resume <run_id> [--from <stage>] <your answers or instructions>`
   - when a branch exists: its name, the checkpoints so far, and that the work is preserved on it — discard with `git checkout master && git branch -D <branch>`, or continue with `--resume`
3. For `scope_change`, say explicitly that resuming returns to planning and needs re-approval. Mention `--from <stage>` for going back further, e.g. to requirements.

## Rules
- **You write only two kinds of file:** files in `run_dir`, and approved documents to validated `publish_to` targets through step 5.1. Personas return content and you save it.
- Never edit any other project file, run tests, write specs, or review code yourself.
- Git: only the run branch and its checkpoints (step 0). Never push. Never squash-merge unless the user asks.
- Never skip, reorder, or add stages, and never invent or change a verdict.
- Save `state.json` after every state change so any run can be resumed.
- Keep chat updates short. The detail lives in the artifacts.
