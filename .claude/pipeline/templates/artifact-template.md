---
run_id: <run id, from the orchestrator>
stage: <stage id, from the orchestrator>
persona: <persona name>
attempt: <attempt number, from the orchestrator>
verdict: pass | fail | changes-requested | blocked
scope_change: false
summary: <one line>
spec_id: <optional. Required when the stage's publish_to uses {spec_id}; kebab-case>
---

# <Stage title>

<!--
Two body layouts:

1. Regular stages: use the sections below.

2. Publishing stages (the workflow sets publish_to): write the document itself first, then a final
   "## Pipeline notes" section containing "### Plan", "### Feedback for next stage" and "### Questions / blockers".
   After approval, only the part ABOVE "## Pipeline notes" is published.

Requirement IDs: REQ-n for requirements, AC-n.m for acceptance criteria, T-n for design tasks.
Tags: [baseline] (existing behavior, inferred from code), [changed], and "Withdrawn <date>: <reason>" (never reuse IDs).
-->

## Plan
<What you decided to do before doing it. On re-runs, how you are addressing the feedback.>

## Result
<Persona-specific body.>

## Feedback for next stage
<Required for fail / changes-requested: concrete, numbered, actionable items. Otherwise "None".>

## Questions / blockers
<Required when verdict is blocked or scope_change is true. Otherwise "None".>
