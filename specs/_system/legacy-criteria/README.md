# Legacy acceptance criteria

The acceptance criteria that mango's older `// AC-<group>.<n>` test comments refer to, recovered on 2026-09-17 under [SPEC-2](../backlog.md#spec-2).

They were written by dev-pipeline runs into `.dev-pipeline/runs/**/01-plan.v*.md`, which is gitignored, so until now **nothing in this repository defined them** and every tag in `src/` and `tests/` resolved to nothing from a clone. 201 criteria across 48 requirements are recovered here, verbatim, with the test that verified each one where the run's test report recorded it.

These predate the spec-scoped `AC-<spec-id>.<n>` convention that `specs/constitution.md` now requires. New criteria do not go here; they go in `specs/<spec-id>/requirements.md`.

## The files

| File | Covers | Groups | Criteria |
|---|---|---|---|
| [batch-1-trustworthy-build](batch-1-trustworthy-build.md) | Batch 1: make `mango build` trustworthy | `AC-1`–`AC-6` | 25 |
| [batch-2-consistent-output](batch-2-consistent-output.md) | Batch 2 — consistent, correct output | `AC-1`–`AC-9` | 45 |
| [batch-3-usable-site](batch-3-usable-site.md) | Batch 3 — make it a usable site | `AC-1`–`AC-7` | 45 |
| [batch-4-tags-feed-sitemap](batch-4-tags-feed-sitemap.md) | Batch 4 - tags, RSS feed and sitemap | `AC-1`–`AC-9` | 43 |
| [risk-2-symlinked-asset-folders](risk-2-symlinked-asset-folders.md) | RISK-2 — symlinked folders inside the assets folder | `AC-10` | 12 |
| [risk-1-symlinked-content-folders](risk-1-symlinked-content-folders.md) | Reject symlinked folders in the site folder (RISK-1) | `AC-11` | 15 |
| [test-1-crlf-line-endings](test-1-crlf-line-endings.md) | Pin CRLF line-ending behavior with tests (TEST-1) | `AC-12` | 16 |

## A numeric ID alone does not identify a criterion

The four `tasks.md`-driven batches each numbered their criteria from `AC-1`, so most low group numbers mean several different things. This table is the ambiguity, exactly:

| Group | Defined by | Ambiguity |
|---|---|---|
| `AC-1` | `batch-1-trustworthy-build`, `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **4-way** |
| `AC-2` | `batch-1-trustworthy-build`, `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **4-way** |
| `AC-3` | `batch-1-trustworthy-build`, `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **4-way** |
| `AC-4` | `batch-1-trustworthy-build`, `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **4-way** |
| `AC-5` | `batch-1-trustworthy-build`, `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **4-way** |
| `AC-6` | `batch-1-trustworthy-build`, `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **4-way** |
| `AC-7` | `batch-2-consistent-output`, `batch-3-usable-site`, `batch-4-tags-feed-sitemap` | **3-way** |
| `AC-8` | `batch-2-consistent-output`, `batch-4-tags-feed-sitemap` | **2-way** |
| `AC-9` | `batch-2-consistent-output`, `batch-4-tags-feed-sitemap` | **2-way** |
| `AC-10` | `risk-2-symlinked-asset-folders` | unique |
| `AC-11` | `risk-1-symlinked-content-folders` | unique |
| `AC-12` | `test-1-crlf-line-endings` | unique |

## Resolving a tag in the code

A tag such as `// AC-7.3` in `src/` or `tests/` cannot be resolved by its number alone. Use the **test name**, which is unambiguous:

1. Take the name of the test the comment sits on, e.g. `covers_every_item_kind_sorted`.
2. Search the `## Verified by` tables in this folder for that name.
3. The file that lists it is the run that defined the criterion; the ID then resolves within that file.

Where a test has been renamed since, `git log -S'<the tag line>' -- <file>` gives the commit that introduced the tag, and its date places it in one run — the run ids are in each file's header.

Rewriting the tags themselves into the spec-scoped form, so this lookup is no longer needed, is [SPEC-3](../backlog.md#spec-3). It is deliberately scheduled after ARCH-1, which will move and delete many of the tag sites.

## Fidelity

Requirements sections are copied verbatim from each run's **latest** plan artifact, which is the version that was approved; earlier rejected versions are not included. The `## Verified by` tables come from each run's latest test report and name tests as they were at that time. Nothing here has been rewritten or summarised, so a criterion that was wrong or later withdrawn is still recorded as written — this is a record of what was specified, not a statement of current behavior. Current behavior is `CLAUDE.md`, `README.md` and the tests.
