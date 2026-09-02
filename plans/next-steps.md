# Next steps

The execution order. One table, short on purpose: this is the file to reread
weekly, and the file an agent reads to know what is next.

A plan not listed here is written, valid, and off the critical path until its
precondition exists. Park it in the second table rather than deleting it.

| # | Work | Exit condition |
| --- | --- | --- |
| 1 | [Adoption is proven end to end](adoption-is-proven-end-to-end.md) | `L3.GATE_HAS_FRESH_EVIDENCE` is active on this repository against a sealed run of `sf init` on a codebase nobody tuned it for, and goes red the next time `src/init.rs` changes without the run being repeated. |

| 2 | [The gate assumes a way to run the product](the-gate-assumes-a-way-to-run-the-product.md) | This repository's `adoption` evidence comes from a harness the tool generated, the gate is sealed against it and `sf check` is green, and the same generator run against a repository nobody tuned it for writes a harness whose report goes red when that product is broken. |

| 3 | [Expand the language adapters](expand-language-adapters.md) | A repo in a new language runs `sf init`, `sf verify` is green, and `sf check` finds something a maintainer of that language agrees is real. |

| 4 | [The four L0 structural rules assume an import statement](structural-rules-assume-an-import-statement.md) | `sf check` on a Rails repository with `languages: [ruby]` reports a real cross-layer finding read from a constant reference rather than a `require` statement, with zero findings on Rails' own base-class inheritance. |

| 5 | [The grain has a ceiling and no floor](the-grain-has-a-ceiling-and-no-floor.md) | `sf check` on a repository nobody tuned it for names at least one indirection site whose own maintainer agrees should not exist, and stays silent on `software-factory`'s own `src/`, where every candidate hit measured so far is correct code. |

| 6 | [A plan bigger than its proofs](a-plan-bigger-than-its-proofs.md) | `sf check` on this repository names the two plans whose criteria are entirely debt and the one plan with no criteria at all, stays silent on `rules-activate-by-dependency-version.md`, and `sf verify` proves the rule fires on both a floor fixture and a ceiling fixture. |

| 7 | [A gate bigger than its proofs](a-gate-bigger-than-its-proofs.md) | A gate requiring no assertions, over a report carrying none, with an emptied goal denylist, turns `sf check` red, and this repository's own `adoption` gate stays green. |


## Parked

| Work | Waiting on |
| --- | --- |
| [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | Version-conditional activation (#3). A pack that cannot say which upstream version it describes is a pack somebody has to remember to remove, which is the state it exists to fix. |
