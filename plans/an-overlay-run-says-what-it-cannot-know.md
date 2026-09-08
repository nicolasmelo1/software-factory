# An overlay run says what it cannot know

`sf check --policy <dir>` shipped in `c8a1db6`. The read path works, the two
refusals work, and every report format names the overlay's path and digest and
states that the run carried no ratchet. What did not ship is the half that plan
called inapplicability, and without it the feature misreports.

## Measured

A bare repository, one Rust file, no `.software-factory/` of its own, governed
by this repository's policy directory from outside. Eighty-odd findings, and
the code under check earned none of them:

- `L4.EVERY_RULE_HAS_A_WHY` reports every enabled rule as "enforced but never
  explained in prose". The prose is in the repository the policy lives in, and
  `docs.scan` was resolved against the target.
- `L5.EVERY_CHECK_HAS_A_MUTATION_TEST` reports every enabled rule as "enabled
  with nothing proving it ever fires". The fixtures are under
  `.software-factory/mutations/`, inside the overlay.
- `L3.GATE_COVERS_THE_PLAN` reports that gate `adoption` names a plan that does
  not exist. The gate's criteria document is in the overlay's repository, which
  is exactly where [a gate outlives its
  plan](../docs/design/a-gate-outlives-its-plan.md) put it.
- `L2.DEPENDENCIES_CHANGE_DELIBERATELY` reports that its lock has never been
  written, naming a path inside a repository that carries no factory directory.
  `sf lock` is refused under an overlay by design, so nothing can ever clear
  this one.
- `L4.ROOT_FILES_ARE_DECLARED`, `L2.CATALOG_ONLY_TIGHTENS` and
  `L5.NO_INERT_RULE` fail the same way, over the root allowlist, the catalog
  fingerprint, and a plans scope that matches nothing.

`inapplicable_under_overlay` in [`src/checks/mod.rs`](../src/checks/mod.rs)
names three rules: `L3.GATE_HAS_FRESH_EVIDENCE`, `L2.POLICY_ONLY_TIGHTENS` and
`L2.FACTORY_CONFIG_IS_LOCKED`. Every other rule that reads state out of a
factory directory runs against the target's, which is not there, and a missing
file reads as a violation.

## Why a hand-written list of three is the defect

The set was written by naming the rules somebody thought of. That is a list
that goes stale on the next rule, and the three above are not a category, they
are a memory. What the rules have in common is structural: the state they read
lives beside the policy, not beside the code. A gate's evidence, a mutation
fixture, a hash lock, a catalog fingerprint, the prose that explains a rule,
the queue a plan sits in. Derived from the check kind and the paths it reads,
the set answers for rules nobody has written yet.

## The one judgment call, decided

`L4.ROOT_FILES_ARE_DECLARED` reads `.allowed-root-files` at the target's root,
not inside a factory directory, so a target could carry one. It is still
inapplicable under an overlay: clearing the finding means writing a file into a
repository the overlay is not allowed to write to, and a finding whose only fix
is refused is not advice. The same reasoning covers any future rule whose
remedy is a file `sf init` writes.

`L5.NO_INERT_RULE` is different, and composes rather than joins the set: it
reports on other rules, so it skips the ones already reporting inapplicable
instead of reading their silence as inertness.

## The three proofs the parent plan named and never got

Its criteria cited `test:src/policy.rs` for the refusal against a root with its
own policy, `test:src/main.rs` for the writers refusing the flag, and
`test:src/verify.rs` for the overlay's own rules being provable. None of the
three exists. The refusal lives inline in `cmd_check`, `src/main.rs` has no
test module at all, and the writers are refused by clap rather than by anything
asserted. Behaviour that is right today and untested is behaviour that is right
today.

## Acceptance criteria

- [ ] Every rule whose state lives beside the policy reports inapplicable
      under an overlay with its reason, and the set is derived from the check
      kind and the paths it reads rather than from a list of ids
      (proof: test:src/checks/mod.rs)
- [ ] An overlay run over a repository carrying no factory directory reports
      what the same policy reports vendored into it over the same code,
      differing only in the ratchet
      (proof: test:src/checks/mod.rs)
- [ ] `L5.NO_INERT_RULE` skips a rule that is inapplicable under the overlay
      rather than reporting it inert
      (proof: test:src/checks/mod.rs)
- [ ] `L4.ROOT_FILES_ARE_DECLARED` is inapplicable under an overlay, on the
      reasoning decided above
      (proof: test:src/checks/mod.rs)
- [ ] `--policy` against a root that carries its own policy is refused by a
      test, not only by a message
      (proof: test:src/policy.rs)
- [ ] `sf init`, `sf ratchet`, `sf lock`, `sf fixtures` and `sf seal` are
      proven to refuse the flag, read out of the clap definition rather than
      asserted in a comment
      (proof: test:src/main.rs)
- [ ] `sf verify` proves the overlay's own rules fire in the repository the
      overlay lives in, so governing from outside cannot ship rules nothing
      trips
      (proof: test:src/verify.rs)

**Exit condition:** an overlay run over a repository that carries no
`.software-factory/` reports the findings that code earns and nothing else,
every rule whose state travels with the policy says so per rule with its
reason, the same policy vendored into that repository reports the same
findings, and each refusal the flag owes carries a test.
