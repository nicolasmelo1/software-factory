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
- **A rule needs a mutation fixture, in every language it claims.** Add it to
  `src/fixtures.rs` in the same change, then `sf fixtures`.
  `L5.EVERY_CHECK_HAS_A_MUTATION_TEST` fails without one, and `sf verify` fails
  both if the fixture does not trip the rule and if it trips it in only some of
  the languages the rule declares a query for.
- **A rule must be pointed at something, or switched off in writing.**
  `L5.NO_INERT_RULE` fails an enabled lock with no scope, a hazard rule with no
  tools, or a structural rule with no query for any language this repository
  declares. Disabled with a reason in `docs/rules.md` is honest; enabled and
  inert is a rule lying about its own coverage.
- **Never widen a rule to make a finding disappear.** If a rule is wrong, argue
  it in the prose and change it deliberately. Silently loosening a glob is
  indistinguishable from a fix at the diff level, which is exactly the failure
  mode this repository exists to catch.
- **Never hand-edit a digest or a lock.** Run `sf lock` or `sf seal <gate>`.
- **Never weaken the policy to go green.** `L2.FACTORY_CONFIG_IS_LOCKED` will
  notice the edit and `L2.POLICY_ONLY_TIGHTENS` will notice its direction. If a
  rule genuinely needs loosening, that is a pull request about the rule, with
  the reasoning in the body — not a line inside a change about something else.
- **Order of operations after touching the guardrail:** `sf fixtures`,
  `sf docs`, `sf ratchet`, then `sf lock` last. The lock covers the ratchet, so
  locking before re-seeding leaves the build red.
- **Rule ids are a public contract.** Every repository that adopted this tool
  pinned them. Renaming one is a breaking change.

## Adding an interview decision or a template

`interview/decisions.yaml` is the decision tree and `templates/` are the
parameterised rules it instantiates. Both are data on purpose: the mapping from
an answer to a rule must not be a judgement any individual agent gets to make,
or two people interviewing the same team end up with different policies.

A template carries its own `fixture:` block and is validated when filled in, so
a template that produces an unrunnable rule fails at `sf init` rather than in
somebody's repository. Placeholders are `@@name@@`, never `${name}` — fixtures
contain real source in four languages and `${...}` is a template literal in two
of them.

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
