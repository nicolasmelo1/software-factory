# A gate bigger than its proofs

[A plan bigger than its proofs](a-plan-bigger-than-its-proofs.md) closed a floor
at the plan layer: a plan listed in the execution order declares at least one
acceptance criterion, because any ceiling expressed as a ratio is evaded by
deleting the numerator.

The gate layer has the same shape and no floor at all.

## Measured

A minimal repository, `sf 0.2.0`, one gate, one sealed manifest, one report:

```yaml
gates:
  checkout:
    activation: ["src/**"]
    evidence: "evidence/checkout.json"
    required_assertions: []
rules:
  L3.GATE_HAS_FRESH_EVIDENCE:
    enabled: true
    options:
      forbidden_in_goal: []
```

```json
{ "scenario": "checkout", "status": "passed",
  "goal": "/Users/someone/code/shop/src/checkout — replay the recorded steps",
  "assertions": [] }
```

`sf check` reports `✓ 1 rules, no findings`, exit 0.

Every one of the five properties `docs/method.md` claims for L3 is satisfied and
none of them is doing anything. The gate activated. The manifest was
re-verified. Nothing was `unsupported`, because nothing was asserted. The
implementation digest matched. And the goal — an absolute path into somebody's
source tree, which the method names as the definition of a replay recipe rather
than a customer — passed, because the list that would have caught it was emptied
in policy.

This is not the check failing. Each of its clauses did exactly what it says.
It is the check having no floor, three separate ways.

## The three

**A gate that requires nothing.** `required_assertions` is the union of what
policy demands and what the manifest admits it owed, which is the right design
against under-declaration and says nothing about the union being empty.
`L3.GATE_COVERS_THE_PLAN` covers part of this and only part: it reads a gate's
`plan`, and `plan` is an `Option`, so a gate that names none owes nothing. Even
when it names one, a plan whose criteria are all `deferred:` or `unspecified:`
requires nothing of the gate — the debt is honest at the plan layer and arrives
at the gate layer as coverage.

**A report that asserts nothing.** `check_assertions` iterates
`required_assertions` and separately sweeps for `unsupported`. Both loops are
empty when the report carries no assertions, so a run that observed nothing and
a run that observed everything are the same green.

**A denylist emptied in policy.** `checks::tightening::option_size` compares
exactly three things across a policy change: `exclude.len()`, `scope.len()` and
`max`. `forbidden_in_goal` is in neither, so emptying it is invisible to
`L2.POLICY_ONLY_TIGHTENS` — the silent loosening that rule exists to price, on
the option that carries L3's fifth property. `forbidden_actors`, added alongside
the actor check, inherits the same hole; it is one more instance of an existing
gap rather than a new one, and it should not be closed on its own.

## Why this repository is a negative control

`software-factory`'s own `adoption` gate names a plan, requires four assertions,
and the plan cites all four with no debt. So it demonstrates the shape working
and cannot demonstrate the shape failing, exactly as the grain plan, shipped in
`8cc43fb`, recorded for its own metric. The fixture is where the failing shape
has to live.

## What is not decided here

Whether the floor is one rule or three. The first two are properties of the same
run and could be one check with two messages; the third is about policy movement
and belongs in `L2` rather than `L3`, which would make it a change to
`option_size` and not a new rule at all. Splitting it wrongly is cheap to do and
expensive to undo, because rule ids are a public contract.

Also undecided: whether a gate with no `plan:` should be a finding on its own.
It is defensible that a gate is allowed to exist before the plan that justifies
it, and defensible that this is precisely how a gate ends up requiring nothing.

## Decided

The two L3 floors stay in their existing rule identities. A plan that gives a
named gate only `deferred:` or `unspecified:` criteria cannot leave that gate's
`required_assertions` empty; this belongs to `L3.GATE_COVERS_THE_PLAN`, because
the question is whether the plan gives its gate anything to cover. A gate with
no `plan:` remains allowed: it may exist before the work that justifies it is
written, and turning that absence into a finding would be a separate policy
decision.

A report with zero assertions is distinct evidence debt and belongs to
`L3.GATE_HAS_FRESH_EVIDENCE`. Its per-assertion loops otherwise faithfully
compare nothing and report success, even when the gate and its manifest both
require nothing.

The two policy denylists join `L2.POLICY_ONLY_TIGHTENS`, rather than creating a
third L3 rule. Removing entries from `forbidden_in_goal` or
`forbidden_actors` is a policy movement, and the existing L2 comparison is the
place that already makes every other quiet weakening visible.

## Non-goals

`L3.GATE_HAS_FRESH_EVIDENCE`'s existing five clauses are not touched. Nothing
here argues for weakening the actor denylist or the goal denylist; the argument
is that emptying them should cost something, which is the opposite direction.

Raising the debt at the plan layer is out of scope. `deferred:` and
`unspecified:` are the mechanism that keeps debt visible and this plan depends
on them continuing to work.

## Acceptance criteria

- [x] The split above is decided and written into `docs/rules.md` before any
      check ships: one rule or three, and whether the policy-movement half lands
      in `L2` instead
      (proof: unspecified:the rule identities are a public contract; the
      decision and its rationale are recorded under "Decided")
- [x] A gate whose `required_assertions` is empty and whose plan cites no
      undeferred criterion is a finding
      (proof: test:.software-factory/mutations/L3.GATE_COVERS_THE_PLAN/)
- [x] A run whose report carries zero assertions is a finding, distinct from the
      one above, so a gate that demands nothing and a report that observed
      nothing do not hide behind each other
      (proof: test:src/checks/evidence.rs)
- [x] Emptying `forbidden_in_goal` or `forbidden_actors` across a policy change
      is reported by `L2.POLICY_ONLY_TIGHTENS`
      (proof: test:src/checks/tightening.rs)
- [x] `sf verify` proves whatever ships fires on its own mutation fixture, and
      the fixture carries the failing shape rather than the empty-map shape
      (proof: test:.software-factory/mutations/L3.GATE_HAS_FRESH_EVIDENCE/)
- [x] `software-factory`'s own `adoption` gate stays green throughout, since it
      is the negative control
      (proof: test:.software-factory/evidence/adoption-scenario.sh)

**Exit condition:** the minimal repository measured above — a gate requiring no
assertions, a report carrying none, and an emptied goal denylist — turns
`sf check` red, and `software-factory`'s own `adoption` gate stays green.
