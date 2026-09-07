# Language coverage

A structural rule reaches a language when it carries a tree-sitter query for
it. A rule with no query for a language does not apply there, which is the
correct outcome rather than a gap to paper over.

Rules that read text, count branches or run a command are not in the table
below: they apply to every language the policy declares.

## Which rules carry a query, and where

<!-- sf:generated language-coverage -->
| Rule | Languages with a query |
| :-- | :-- |
| `L0.EXCEPTIONS_HAVE_ONE_HOME` | go, python, ruby, rust, typescript |
| `L0.NO_CROSS_LAYER_IMPORT` | go, python, ruby, rust, typescript |
| `L0.ONE_ENTRYPOINT_PER_FILE` | go, python, typescript |
| `L0.PERSISTENCE_STAYS_IN_REPOSITORIES` | go, python, typescript |
| `L1.INDIRECTION_EARNS_ITS_NAME` | python, rust, typescript |
| `L1.SKIPPED_TESTS_STATE_A_REASON` | go, python, ruby, rust, typescript |
| `L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK` | go, python, ruby, rust |
| `L6.ONE_LOCK_AT_A_TIME` | go, python, ruby, rust |

<!-- sf:end language-coverage -->

## Grammars this binary carries

<!-- sf:generated language-grammars -->
| Grammar | Files it reads |
| :-- | :-- |
| python | `*.py`, `*.pyi` |
| typescript | `*.ts`, `*.mts`, `*.cts` |
| typescript | `*.tsx` |
| go | `*.go` |
| rust | `*.rs` |
| ruby | `*.rb`, `*.rake`, `*.gemspec`, `*.ru` |

<!-- sf:end language-grammars -->

`tsx` shares the TypeScript rule surface: a rule written for `typescript`
applies to `.tsx` too, or half a React repository goes unchecked.

Adding a language is a grammar in `src/lang.rs`, giving the node kinds that
open a function, the ones that branch, and the boolean operators, plus one
query per rule you want it to cover.

`sf verify` requires every language a rule declares to be shown tripping it,
otherwise three broken queries hide behind one that works.
