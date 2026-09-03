# Next steps

The execution order. One table, short on purpose: this is the file to reread
weekly, and the file an agent reads to know what is next.

A plan not listed here is written, valid, and off the critical path until its
precondition exists. Park it below rather than deleting it.

| # | Work | Exit condition |
| --- | --- | --- |
| 1 | [The gate assumes a way to run the product](the-gate-assumes-a-way-to-run-the-product.md) | This repository's `adoption` evidence comes from a harness the tool generated, the gate is sealed against it and `sf check` is green, and the same generator run against a repository nobody tuned it for writes a harness whose report goes red when that product is broken. |

| 2 | [Expand the language adapters](expand-language-adapters.md) | A repo in a new language runs `sf init`, `sf verify` is green, and `sf check` finds something a maintainer of that language agrees is real. |

| 3 | [The four L0 structural rules assume an import statement](structural-rules-assume-an-import-statement.md) | `sf check` on a Rails repository with `languages: [ruby]` reports a real cross-layer finding read from a constant reference rather than a `require` statement, with zero findings on Rails' own base-class inheritance. |

| 4 | [A plan bigger than its proofs](a-plan-bigger-than-its-proofs.md) | `sf check` on this repository names the four plans over the ceiling and the one plan with no criteria at all, stays silent on `rules-activate-by-dependency-version.md`, and `sf verify` proves the rule fires on both a floor fixture and a ceiling fixture. |

| 5 | [A gate bigger than its proofs](a-gate-bigger-than-its-proofs.md) | A gate requiring no assertions, over a report carrying none, with an emptied goal denylist, turns `sf check` red, and this repository's own `adoption` gate stays green. |

| 6 | [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | `sf pack add <name>@<version>` vendors a versioned set of rules with their fixtures into `.software-factory/rules/`, refuses any pack whose fixtures do not trip its own rules, and the installed rules deactivate with a finding when the dependency's major version moves. |


## Parked

Nothing is parked. Rule packs waited on version-conditional activation, which
shipped in `927e8e2`, so they join the order above.


## Shipped, kept

Done, and still on disk because something points at it. Nobody is queued to
work on these.

| Plan | Why it stays |
| --- | --- |
| [Adoption is proven end to end](adoption-is-proven-end-to-end.md) | `gates.adoption.plan` names it in `.software-factory/policy.yaml`, and `L3.GATE_COVERS_THE_PLAN` reports a gate whose plan does not exist. |
| [Rules activate on the version of the dependency they are about](rules-activate-by-dependency-version.md) | Shipped in `927e8e2`. It is the negative control named in the exit condition of [A plan bigger than its proofs](a-plan-bigger-than-its-proofs.md). |
