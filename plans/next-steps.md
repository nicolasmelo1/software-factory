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

## Parked

| Work | Waiting on |
| --- | --- |
| [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | Version-conditional activation (#2). A pack that cannot say which upstream version it describes is a pack somebody has to remember to remove, which is the state it exists to fix. |
