# A gate outlives its plan

Delivered. This document was `plans/a-gate-outlives-its-plan.md`, the plan this
repository executed for it; it now lives here as the design note the gate's
criteria belong in, which is the arrangement the plan argued for. The
arguments below are the plan's, kept verbatim except where the work changed a
fact; the acceptance criteria at the bottom carry their original proof
markers, now checked.

A plan is a thing you are about to do. A gate is a thing that stays true.

One gate in `.software-factory/policy.yaml` names a plan as the document its
criteria live in, and that welds the two together. `gates.adoption.plan` points
at [`docs/design/adoption-is-proven-end-to-end.md`](adoption-is-proven-end-to-end.md),
whose work is finished, and the file cannot be deleted: taking it away takes
the gate's criteria with it and `L3.GATE_COVERS_THE_PLAN` goes red.

`amy` reached this with seven gates and nine delivered plans, at which point
its execution order read as history and the one question the directory exists
to answer, what is next, cost more to answer every time something shipped. Here
it is one gate and two delivered plans, and `plans/next-steps.md` already
carried the table where the archive forms: **Shipped, kept**, with a "Why it
stays" column. That column is the tell. A document kept because deleting it
would break a check is a document nobody is reading on purpose.

The cheap moment to unweld this is now, while it is one line of policy rather
than seven.

## Where the criteria belong

`gate_coverage` joins the root and whatever path `gates.<name>.plan` gives it
([`src/checks/cadence.rs`](../../src/checks/cadence.rs)), and requires nothing
about that path being under `plans/`. So the criteria can live where the
decision lives.

Each delivered gated plan becomes a design note under `docs/design/`, carrying
its acceptance criteria verbatim, its exit condition, and the paragraph of
argument that says why the gate asserts what it asserts. `gates.<name>.plan`
moves to that path, `sf lock` records the new policy hash, and the plan file is
deleted. `plans/next-steps.md` lost its **Shipped, kept** table.

This sits directly after
[the documentation design note](docs-are-read-out-of-the-code.md) on purpose.
That note decides how prose in `docs/` is written and guarded, and defers a
navigation file until the page count justifies one. A design-note group is
three or four more pages, which is most of that argument.

## The consequence nobody notices until it bites

Three L4 rules are scoped to `plans/*.md`, and criteria that move out of
`plans/` move out of two of them.

`L4.PLAN_CRITERION_NAMES_ITS_CHECK` and `L4.PLAN_PROOF_BUDGET` have to name
the design notes in their scope, or a criterion in a design note stops needing
a proof marker and the debt in one stops counting. That is the whole guarantee
the join depends on.

`L4.PLAN_DECLARES_EXIT_CONDITION` must **not** be extended, because it
requires every document in its scope to be listed in the execution order or
parked there. A design note is neither. Extending it would put delivered work
back in the queue, which is the problem this plan is about, arriving from the
other direction.

## What makes the move safe to attempt

`L4.DOC_LINKS_RESOLVE` covers `**/*.md`, so a delivered plan cannot be deleted
while anything still links to it: the check names the dangling link before the
commit lands. The tool already holds the hard half of this migration.

## The rule that keeps it that way

`amy` needed a script of its own here, because nothing in `sf` can see the
invariant. We are `sf`, so it ships as a rule and every adopting repository
gets it: a new L3 rule that reports a gate whose criteria document sits inside
the queue of undone work. Its id is chosen when it lands and not here,
because a rule id is a public contract and `L4.EVERY_RULE_HAS_A_WHY` reports a
citation the catalog cannot resolve. This paragraph named one in its first
draft, and the check said so.

It carries the full cost of a rule in this repository, and that is the honest
estimate: `why` and `fix` in the catalog entry, a mutation fixture in
[`src/fixtures.rs`](../../src/fixtures.rs) that a gate pointed back into
`plans/` trips, a section in [`docs/rules.md`](../rules.md), then `sf fixtures`,
`sf docs`, `sf ratchet` and `sf lock`, in that order.

It also inherits L3's inertness problem. A repository that declares no gate
cannot produce a finding from this rule, and a rule that cannot fire has to say
so rather than report green, which is the path `inert_gate_coverage_reason`
already walks for `L3.GATE_COVERS_THE_PLAN`. This one has a third door of its
own: the queue this rule measures against is what the policy declares as
`docs.plans_dir`, and a policy that declares none leaves the rule's predicate
unevaluable — reported as inertness rather than read as coverage.

From then on the handover is mechanical. Work lands, its criteria move into the
design note the gate cites, the plan file goes, and
`L3.GATE_COVERS_THE_PLAN` holds those assertions against a document that has no
reason to be deleted.

## What the work decided

- The rule shipped as `L3.GATE_PLAN_NOT_IN_THE_QUEUE`, reading
  `docs.plans_dir` for the queue and reporting inert when it is undeclared.
- The `rules-activate-by-dependency-version` argument survives as a design
  note beside this one, and stays the negative control for the plan proof
  budget; the behaviour its fixture already encodes is unchanged.
- The three delivered documents that moved here carry their criteria
  verbatim; `L4.PLAN_CRITERION_NAMES_ITS_CHECK` and `L4.PLAN_PROOF_BUDGET`
  follow them through their policy scope, and
  `L4.PLAN_DECLARES_EXIT_CONDITION` deliberately does not.

## Acceptance criteria

- [x] No gate in this policy names a document under `plans/`, and the new L3
      rule reports one that does
      (proof: test:src/checks/cadence.rs)
- [x] The new rule trips its own mutation fixture, so it is a rule rather than
      a green checkmark
      (proof: test:src/fixtures.rs)
- [x] The rule reports as inert, not green, in a repository that declares no
      gate, on the path `L3.GATE_COVERS_THE_PLAN` already uses
      (proof: test:src/checks/cadence.rs)
- [x] Every assertion the `adoption` gate requires is still named by a
      criterion, now in the design note, with
      `sf check --rule L3.GATE_COVERS_THE_PLAN` green against it
      (proof: test:src/checks/cadence.rs)
- [x] `L4.PLAN_CRITERION_NAMES_ITS_CHECK` and `L4.PLAN_PROOF_BUDGET` name the
      design notes in their scope, and `L4.PLAN_DECLARES_EXIT_CONDITION` does
      not
      (proof: test:src/checks/cadence.rs)
- [x] Deleting a delivered plan leaves no dangling link, proved by
      `L4.DOC_LINKS_RESOLVE` going red first when one is missed
      (proof: test:src/checks/cadence.rs)
- [x] `docs/rules.md` carries the new rule's section, and `sf docs` changes
      nothing after it
      (proof: test:src/init.rs)
- [x] `plans/next-steps.md` has no **Shipped, kept** table, and reads top to
      bottom as work nobody has done yet
      (proof: unspecified:a property of the diff rather than of a run)
- [x] Whether the argument in `rules-activate-by-dependency-version` survives
      as a design note or only as the fixture that already encodes it is
      decided before that file is deleted
      (proof: unspecified:decided — it survives as a design note beside this
      one, and remains the negative control for the plan proof budget)

**Exit condition, met:** `gates.adoption.plan` points at a document under
`docs/design/`, `sf check` is green with the new L3 rule enabled, a gate
pointed back into `plans/` turns it red (the rule's own mutation fixture
proves that direction, and `sf verify` holds it), `plans/` holds only work
nobody has done yet, and `plans/next-steps.md` read top to bottom is a list of
things that have not happened.