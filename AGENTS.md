# Agent instructions

This repository is a software factory harness. It holds itself to the method it
ships.

## Before changing anything

1. Read [`docs/method.md`](docs/method.md). The layering is not arbitrary and
   the ordering (L1/L4/L5 first, L0 after the third occurrence of a pattern) is
   the load-bearing advice.
2. Run `sf verify` before `sf check`. A check that no longer fires makes every
   green run meaningless, and it is the cheaper failure to find.

## Hard rules

- **A rule needs `why` and `fix`.** The catalog refuses to load without them,
  and `L4.EVERY_RULE_HAS_A_WHY` fails when enforcement and prose come apart.
  A new rule means a new section in `docs/rules.md` in the same commit.
- **A rule needs a mutation fixture.** Add it to `src/fixtures.rs` in the same
  change. `L5.EVERY_CHECK_HAS_A_MUTATION_TEST` fails without one, and
  `sf verify` fails if the fixture does not actually trip the rule.
- **Never widen a rule to make a finding disappear.** If a rule is wrong, argue
  it in the prose and change it deliberately. Silently loosening a glob is
  indistinguishable from a fix at the diff level, which is exactly the failure
  mode this repository exists to catch.
- **Never hand-edit a digest or a lock.** Run `sf lock` or `sf seal <gate>`.
- **Rule ids are a public contract.** Every repository that adopted this tool
  pinned them. Renaming one is a breaking change.

## Adding a language

A grammar in `src/lang.rs` (node kinds that open a function, node kinds that
branch, boolean operators) plus one query per rule you want it to cover. Rules
whose vocabulary does not exist in that language simply omit it — a rule with
no query for a language does not apply to it, and that is the correct outcome,
not a gap to paper over.

## Working model

- `cargo build --release`, then `./target/release/sf verify && ./target/release/sf check`.
- Conventional Commits.
- Structural checks are tree-sitter queries. Test one with a fixture before
  believing it: a query that matches nothing is the single easiest bug to ship
  here, and the only thing that catches it is the mutation.
