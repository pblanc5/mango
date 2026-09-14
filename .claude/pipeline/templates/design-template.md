# <Feature title>: Design

Spec ID: `<spec-id>` · Requirements: `<specs dir>/<spec-id>/requirements.md`

## Overview
<The approach in a few sentences.>

## Affected components
| Component / file | Change |
|---|---|
| ... | ... |

## Approach
<How it works. Key decisions and why.>

## Interfaces and data
<Signatures, parameters, formats, schema changes. "None" if not applicable.>

## Alternatives considered
| Option | Why not chosen |
|---|---|
| ... | ... |

## Risks
- ...

## Test strategy
- Suite command: `<command from the constitution>`
- Safety net: <[baseline] ACs in changed code that no existing test covers, and the tasks that add those tests first>
- New and changed behavior: <which tests cover which ACs>

## Tasks
<!-- Ordered. Safety-net tasks first. Files together are the developer's scope. Every non-withdrawn AC must be covered. -->
### T-1 <title>
- Kind: safety-net | implementation | test | docs
- Satisfies: AC-1.1
- Files: `path/a`
- Done when: <observable condition>

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-1 |

## Changelog
| Date | Run | Change |
|---|---|---|
| <YYYY-MM-DD> | <run id or "manual"> | <summary> |
