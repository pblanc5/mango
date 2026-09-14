# Project constitution

Rules and conventions every dev-pipeline persona follows in this project. Edit freely; the pipeline reads this file on every run. In an existing project, `/spec-init` can draft this from the repository for you.

## Specs location
Specs live in `{{SPECS_DIR}}/`:
- one folder per feature: `{{SPECS_DIR}}/<spec-id>/requirements.md` and `design.md`
- `{{SPECS_DIR}}/_system/overview.md`

## Commands
| Purpose | Command | Source |
|---|---|---|
| Test | `<command>` | `<where it's defined, e.g. package.json:12>` |
| Lint | `<command>` | |
| Build | `<command>` | |

## Conventions
- <Convention> (example: `<file>`)

## Non-negotiables
- <Rule every change must respect, e.g. "No new runtime dependencies without approval">

## When to use /spec-feature
- Use `/spec-feature` for: public or interface changes, data or format changes, behavior users rely on, changes across multiple components.
- Use `/ship-feature` for: small local changes, internal refactors covered by tests, docs.

## Related docs
- `<path>`: <what it covers>

## Conflicts found
None

## Changelog
| Date | Run | Change |
|---|---|---|
| <YYYY-MM-DD> | manual | Created from template |
