# Next steps

The execution order. One table, short on purpose: this is the file to reread
weekly, and the file an agent reads to know what is next.

A plan not listed here is written, valid, and off the critical path until its
precondition exists. Park it in the second table rather than deleting it.

| # | Work | Exit condition |
| --- | --- | --- |
| 1 | [Expand the language adapters](expand-language-adapters.md) | A repo in a new language runs `sf init`, `sf verify` is green, and `sf check` finds something a maintainer of that language agrees is real. |

| 2 | [Rules activate on the version of the dependency they are about](rules-activate-by-dependency-version.md) | Changing a dependency's pin in the manifest makes `sf check` fail by naming every rule whose `when` no longer matches, instead of leaving them enabled and inert. |

| 3 | [Close the inert-rule blind spot for TextPattern and Complexity](close-the-inert-rule-blind-spot.md) | `sf verify` fires `L5.NO_INERT_RULE` on a fixture inert only through the `TextPattern` or `Complexity` path, and the built binary run against a real repository whose declared language matches none of its files names the blind `L1.*` rules instead of staying silent. |

| 4 | [Ruby language adapter](ruby-language-adapter.md) | `sf init --language ruby` runs, `sf verify` proves every ruby query it declared, and `sf check` finds real cyclomatic complexity in a Ruby codebase (`~/Sites/pp-team/postpilot`) that reported zero before this. |

| 5 | [Ruby joins the L6 hazard rules](ruby-joins-the-l6-hazard-rules.md) | A Ruby repository's own CI workflow is graded by `sf check` for four of the nine L6 hazard concerns, with no tree-sitter grammar for Ruby, proven against a real Rails application's CI configuration. |

| 6 | [The four L0 structural rules assume an import statement](structural-rules-assume-an-import-statement.md) | `sf check` on a Rails repository with `languages: [ruby]` reports a real cross-layer finding read from a constant reference rather than a `require` statement, with zero findings on Rails' own base-class inheritance. |

| 7 | [The grain has a ceiling and no floor](the-grain-has-a-ceiling-and-no-floor.md) | `sf check` on a repository nobody tuned it for names at least one indirection site whose own maintainer agrees should not exist, and stays silent on `software-factory`'s own `src/`, where every candidate hit measured so far is correct code. |

| 8 | [A plan bigger than its proofs](a-plan-bigger-than-its-proofs.md) | `sf check` on this repository names the two plans whose criteria are entirely debt and stays silent on the two that name their proofs, and `sf verify` proves the rule fires on both a floor fixture and a ceiling fixture. |

## Parked

| Work | Waiting on |
| --- | --- |
| [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | Version-conditional activation (#2). A pack that cannot say which upstream version it describes is a pack somebody has to remember to remove, which is the state it exists to fix. |
