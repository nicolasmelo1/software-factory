# Next steps

The execution order. One table, short on purpose: this is the file to reread
weekly, and the file an agent reads to know what is next.

A plan not listed here is written, valid, and off the critical path until its
precondition exists. Park it in the second table rather than deleting it.

| # | Work | Exit condition |
| --- | --- | --- |
| 1 | [Expand the language adapters](expand-language-adapters.md) | A repo in a new language runs `sf init`, `sf verify` is green, and `sf check` finds something a maintainer of that language agrees is real. |

| 2 | [Rules activate on the version of the dependency they are about](rules-activate-by-dependency-version.md) | Changing a dependency's pin in the manifest makes `sf check` fail by naming every rule whose `when` no longer matches, instead of leaving them enabled and inert. |

## Parked

| Work | Waiting on |
| --- | --- |
| [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | Version-conditional activation (#2). A pack that cannot say which upstream version it describes is a pack somebody has to remember to remove, which is the state it exists to fix. |
