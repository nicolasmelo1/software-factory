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

## Parked

| Work | Waiting on |
| --- | --- |
