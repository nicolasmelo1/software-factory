# A plan bigger than its proofs

Nobody does good work on a task that is too large. The information stops fitting,
the dependency graph stops being holdable, and the loose ends surface in testing
or in production rather than in planning. The whole method here is an argument
that the fix is deterministic enforcement rather than better intentions, and the
plan layer is where a change is still cheap to split. It has no size rule.

`L4.PLAN_DECLARES_EXIT_CONDITION` asks for one line and a row in the execution
order. `L4.PLAN_CRITERION_NAMES_ITS_CHECK` asks each criterion it finds to name
its proof. Neither has an opinion about how much a plan promises, so a plan that
is really three plans passes both.

## Measured, on this repository

Debt is a criterion marked `deferred:` or `unspecified:`, which is to say a
promise nothing yet proves. Retaken over what is left after
`the-grain-has-a-ceiling-and-no-floor.md` shipped in `8cc43fb` and was deleted.
This plan is not in its own table; its own count is under "Where to set it".

| Plan | Lines | Criteria | Debt |
| --- | --- | --- | --- |
| [the-gate-assumes-a-way-to-run-the-product](the-gate-assumes-a-way-to-run-the-product.md) | 165 | 7 | 6/7 |
| [expand-language-adapters](expand-language-adapters.md) | 26 | 0 | n/a |
| [structural-rules-assume-an-import-statement](structural-rules-assume-an-import-statement.md) | 222 | 6 | 6/6 |
| [a-gate-bigger-than-its-proofs](a-gate-bigger-than-its-proofs.md) | 127 | 6 | 5/6 |
| [third-party-rule-packs](third-party-rule-packs.md) | 71 | 4 | 4/4 |
| [adoption-is-proven-end-to-end](adoption-is-proven-end-to-end.md) | 70 | 4 | 0/4 |
| [rules-activate-by-dependency-version](rules-activate-by-dependency-version.md) | 91 | 5 | 1/5 |

Two things fall out. The two longest plans are the two where almost no criterion
names a real proof, and they sit at #1 and #3 in the order, so the work furthest
out is the work least defined. And a plan still declares no criteria at all,
which no amount of retaking has changed: the zero-criteria rows that went away
went away because their plans shipped, not because anybody wrote criteria for
them.

## The floor comes first, or the ceiling is decoration

A plan with zero criteria is green today. `plan_criteria` in
`src/checks/cadence.rs` iterates the criteria it parses, so an empty list
produces an empty finding list, and a 120-line plan promising nothing reads
exactly like a plan whose promises are all proven — which is what
`ruby-language-adapter.md` did for its whole life, up to shipping in `0a285f9`.

That is the `L5.NO_INERT_RULE` shape one level up, and on its own it would be
worth closing. The reason it has to be closed first is narrower: any ceiling
expressed as a ratio is evaded by deleting the numerator. An agent against the
limit drops criteria and goes green, which is the ratchet evasion `L2` exists to
make expensive, reachable here through a rule that never had a floor.

So: a plan listed in the execution order declares at least one acceptance
criterion.

## Why the ratio, and not the line count

Counting markdown lines measures how much someone wrote. The quantity that means
something is the gap between what the plan promises and what anyone has worked
out how to prove.

The objection to counting `deferred:` as debt is that a plan describes unbuilt
work, so of course nothing is proven yet, and the ceiling would block every new
plan on day one. Two plans answer it. `rules-activate-by-dependency-version.md`
names `test:src/policy.rs` and
`test:.software-factory/mutations/L5.NO_INERT_RULE/` for work nobody has
started, and `ruby-joins-the-l6-hazard-rules.md` carried four criteria and no
debt at all before it shipped in `e102548`. Naming where the proof will live
does not require the proof to exist. It requires
knowing what would settle the question, which is exactly the knowledge a plan too
large to hold does not have.

`deferred:` is therefore not a free pass. It means the author could not say where
the proof would go. That is the signal, and it is why the count is
`deferred` plus `unspecified` rather than `unspecified` alone.

## Where to set it

At 60 percent, the rule names `a-gate-bigger-than-its-proofs.md`,
`structural-rules-assume-an-import-statement.md`,
`the-gate-assumes-a-way-to-run-the-product.md` and `third-party-rule-packs.md`
on the ceiling and `expand-language-adapters.md` on the floor, and stays silent
on `rules-activate-by-dependency-version.md` and
`adoption-is-proven-end-to-end.md`. `ratchet: allowlist` freezes those five so
adopting the rule is not a demand to rewrite them the same afternoon.

Four over the ceiling out of seven is more than the two this plan measured when
it was written. Two of the four are plans written since for work nobody has
started, and one is the rule-packs plan that just came off the parked table.
Whether that means 60 percent is the wrong number, or that this repository is
planning further ahead than it can prove, is the first thing to settle when this
work is picked up.

The fix is a split, not a rewrite: the half with proofs becomes the plan that
enters the order now, the rest becomes its own file, parked with the precondition
it waits on. Parked plans are a defensible exception, since a plan is parked
precisely because it is not defined yet, and that is a decision to make when the
rule is written rather than to assume here.

This plan carries three proven criteria and three debt markers, which is 50
percent and under its own ceiling.

## What this cannot do

It cannot tell whether a criterion is honest. A criterion written vaguely enough
to name a proof it does not really have passes, and the only defence is the same
one every L4 rule relies on, which is that the marker has to be written by
someone at the moment of writing the promise.

It says nothing about the size of the resulting change. A plan can clear the
ceiling and still produce a diff crossing every boundary in the repository.
That is a separate rule, it needs `--changed` to have anything to say, and its
inertness path without a base ref has to be visible to `L5.NO_INERT_RULE` before
it is worth shipping. Out of scope here.

## Non-goals

- No task decomposition, no dependency graph, no execution state. `sf` grades
  artifacts and does not orchestrate work.
- No change to `L4.PLAN_CRITERION_NAMES_ITS_CHECK`. Rule ids are a public
  contract and adopters pinned that one against per-criterion behaviour, so the
  floor and the ceiling ship as a new rule that can be enabled on its own.
- No new proof kind. `deferred:` and `unspecified:` already carry the meaning.

## Acceptance criteria

- [ ] A plan matched by the rule's scope with zero acceptance criteria produces
      a finding naming the plan file.
      (proof: test:src/checks/cadence.rs)
- [ ] The ratio of `deferred` plus `unspecified` criteria to total criteria is
      compared against a ceiling read from the rule's `defaults`, not a constant
      in the check.
      (proof: test:src/checks/cadence.rs)
- [ ] `sf verify` fires the rule on two fixtures, one tripping the floor and one
      tripping the ceiling, so neither path ships unproven.
      (proof: test:src/fixtures.rs)
- [ ] `L5.NO_INERT_RULE` reports the new rule when it is enabled with a scope
      that selects no plan file.
      (proof: deferred:the inertness path depends on whether the mode routes
      through `scan::select`, which is the intent and not yet written)
- [ ] Run against this repository at the chosen ceiling, `sf check` names every
      plan the table above puts over it and `expand-language-adapters.md` on the
      floor, and reports nothing for `rules-activate-by-dependency-version.md`
      or `adoption-is-proven-end-to-end.md`.
      (proof: deferred:no check is written yet)
- [ ] No existing plan is edited to clear a finding. The one plan left with no
      criteria, `expand-language-adapters.md`, gains real ones or moves to the
      parked table, and no ceiling already in the catalog or in any policy
      moves as part of this work.
      (proof: unspecified:an absence, enforced by reading the diff, which no
      check here can assert)

**Exit condition:** `sf check` on this repository names every plan the measured
table puts over the ceiling and the one plan with no criteria at all, stays
silent on `rules-activate-by-dependency-version.md`, and `sf verify` proves the
rule fires on both a floor fixture and a ceiling fixture.
