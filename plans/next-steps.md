# Next steps

The execution order. One table, short on purpose: this is the file to reread
weekly, and the file an agent reads to know what is next.

A plan not listed here is written, valid, and off the critical path until its
precondition exists. Park it in the second table rather than deleting it.

| # | Work | Exit condition |
| --- | --- | --- |
| 1 | [Expand the language adapters](expand-language-adapters.md) | A repo in a new language runs `sf init`, `sf verify` is green, and `sf check` finds something a maintainer of that language agrees is real. |

| 2 | [Keep the generated prose in sync with the catalog](keep-generated-prose-in-sync.md) | A rule whose `fix` names a command `sf` does not accept fails `sf check`, proven by a fixture. |

| 3 | [Text renderer shows `expected` even when `actual` is absent](text-renderer-shows-expected-without-actual.md) | A finding produced by any check that sets `expected` without `actual` shows that field in `sf check`'s text output, proven by the `L4.ROOT_FILES_ARE_DECLARED` mutation fixture and a stray-root-file check naming `.allowed-root-files`. |

| 4 | [`L1.SKIPPED_TESTS_STATE_A_REASON` in every language `sf` parses](skipped-tests-say-why-in-every-language.md) | A TypeScript repository with a bare `it.skip` fails `sf check` on it, this repository enables the rule on its own Rust source, and `sf verify` proves it fires in all four languages. |

| 5 | [Claims cite the evidence that proves them](claims-cite-their-evidence.md) | A claim marker in a scoped page naming a gate the policy does not declare fails `sf check`, and a claim whose gate lost its evidence to a code change fails through `L3.GATE_HAS_FRESH_EVIDENCE`, both proven by fixtures. |

| 6 | [Rules activate on the version of the dependency they are about](rules-activate-by-dependency-version.md) | Changing a dependency's pin in the manifest makes `sf check` fail by naming every rule whose `when` no longer matches, instead of leaving them enabled and inert. |

| 7 | [Ruby language adapter](ruby-language-adapter.md) | `sf init --language ruby` runs, `sf verify` proves every ruby query it declared, and `sf check` finds real cyclomatic complexity in a Ruby codebase (`~/Sites/pp-team/postpilot`) that reported zero before this. |

## Parked

| Work | Waiting on |
| --- | --- |
| [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | Version-conditional activation (#6). A pack that cannot say which upstream version it describes is a pack somebody has to remember to remove, which is the state it exists to fix. |
