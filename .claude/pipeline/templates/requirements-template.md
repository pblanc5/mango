# <Feature title>: Requirements

Spec ID: `<spec-id>`

## Summary
<What is being asked for and why, in two to four sentences.>

## Context
<Who needs it, the current situation, related specs and docs.>

## Current behavior
<Only when changing existing, previously unspecified behavior. Scope it to the affected area. Otherwise delete this section.>

### REQ-1 [baseline] <title>
Evidence: `path/to/file:12-30`, test `<name>`
Status: inferred, confirm at approval
- AC-1.1 [baseline] WHEN <trigger> THE <component> SHALL <response>

## User stories
- As a <role>, I want <goal> so that <benefit>.

## Requirements
### REQ-2 <title>
<What and why. No design: no file names, algorithms or data structures.>
- AC-2.1 WHEN <trigger> THE <component> SHALL <response>
- AC-2.2 IF <unwanted condition> THEN THE <component> SHALL <response>

<!--
EARS patterns:
  THE <component> SHALL <response>                            always true
  WHEN <trigger> THE <component> SHALL <response>             event
  WHILE <state> THE <component> SHALL <response>              state
  IF <condition> THEN THE <component> SHALL <response>        errors and edge cases
  WHERE <optional feature> THE <component> SHALL <response>   optional feature

ID rules:
  Never renumber or reuse IDs. New items take the next number.
  Changed item: keep the ID, tag [changed], and add a line "Previously: <old behavior>".
  Removed item: ~~AC-2.3 ...~~ Withdrawn <date>: <reason>
-->

## Out of scope
- ...

## Open questions
None

## Changelog
| Date | Run | Change |
|---|---|---|
| <YYYY-MM-DD> | <run id or "manual"> | <summary> |
