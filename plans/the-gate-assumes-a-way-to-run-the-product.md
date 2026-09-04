# The gate assumes a way to run the product

L3 verifies a report of a run against the real thing. Nothing in `sf` produces
that run, and nothing checks that a way to produce it exists.

The five properties `docs/method.md` claims for L3 are all properties of the
report: it is re-hashed, its assertions are re-checked, its digest expires, its
goal and actor are read for leaked answers. Every one of them starts after
somebody has already got the product running. That step is the expensive one,
and it is the one the tool says nothing about.

## Measured

- `sf init` writes `gates: {}` into every policy it scaffolds
  (`src/init.rs:367`). An adopter who enables L3 gets an empty map and no first
  gate.
- The interview's thirteen decisions never mention L3 or a gate
  (`interview/decisions.yaml`). There is no answer that turns the layer on, so
  the only route to a gate is hand-writing four things at once: the policy
  entry, the manifest, the report, and whatever produced the report.
- `skills/factory-evidence/SKILL.md` step 3 is the entire product surface for
  producing a run: "Start the actual entry point a customer would use. Drive it
  the way they would."
- One worked example exists anywhere, and it is in this repository:
  `.software-factory/evidence/adoption-scenario.sh`, 4.5 KB of shell. Nothing
  generated it, nothing regenerates it, and no rule reads it. It is a file we
  happen to keep.

So the layer that exists to prove the product works assumes a way to run the
product, and ships none. We solved that once, by hand, for ourselves. An
adopter meets it as a sentence in a skill.

## Prior art

pstack ships `create-verification-skill`: it interviews the codebase and writes
`.claude/skills/verify-<app>/SKILL.md` carrying launch, doctor, drive, evidence
and cleanup, plus a `features/` directory with one file per user-facing feature
and its observable success criteria, then runs the loop once to prove what it
wrote works. On a multi-tenant application it reportedly worked out how to start
Clerk locally, create an account, an app and an instance, and carried on from
there.

Two things there are right for us and one is not.

**Right: the discovery is expensive and belongs in the repository.** Working out
how to bring a multi-tenant stack up is a half hour that should be spent once,
not once per run. Redoing it per run is also the incentive that produces a
beautifully written report of a run that never happened.

**Right: a feature with an observable success criterion is the same object as a
required assertion.** Their feature map and our `required_assertions` are two
spellings of one list.

**Wrong for us: the artifact is markdown.** Markdown is not reproducible, not
hashable, and drifts in silence, which is why pstack needs a second skill,
`maintain-verification-skill`, to repair it. Our digest already solves that for
anything the gate points at, and only for things that are files with content
somebody runs.

## The shape

1. **The artifact is a program, not a runbook.** It takes the repository,
   launches the real entry point, drives it, and emits the JSON report the
   manifest cites. `adoption-scenario.sh` is the reference shape, including its
   own header: a harness is not the actor, and
   `L3.GATE_HAS_FRESH_EVIDENCE` already refuses a manifest that credits the run
   to the harness.
2. **It comes out of an interview, not a template.** No template knows how to
   start somebody's product. `interview/decisions.yaml` and `sf init --answers`
   are the mechanism that already exists for turning answers into
   configuration, and a second mechanism beside it is a second thing to keep
   true.
3. **The observable criteria land in policy as `required_assertions`,** not in a
   prose directory. `L3.GATE_COVERS_THE_PLAN` reads that list. It cannot read a
   markdown feature map, and a list two checks disagree about is worse than one
   list.
4. **The harness sits inside the gate's activation paths.** Then editing the way
   the product is driven expires the evidence, and re-running is the only fix,
   which is the property the whole layer turns on.

## This repository is the fixture

`adoption-scenario.sh` was written by hand before any of this existed, and it is
correct: it walks the README quickstart against a repository nobody tuned the
tool for, writes one new violation, and checks that the build refuses it. That
makes it the test.

Run the generator on this repository. It has to produce a harness that emits a
report carrying the same four `adoption` assertions, and `sf seal adoption`
against that report has to leave the gate green. If the generator cannot
reproduce the one verifier we already know is right, it writes runbooks and is
not a feature.

That swap is also the end-to-end test of `sf` itself. Today adoption is proven
by a script we maintain next to the tool. After this, it is proven by a harness
the tool produced, running the tool, on a codebase that never heard of it. The
tool is then inside its own loop, which is the only version of this that keeps
working when nobody is watching.

## What is not decided here

**Decided: the existing `factory-evidence` skill.** A subcommand has to embed a
language-neutral notion of "launch this product", which the tool does not have
and cannot acquire by parsing. The skill reads the repository, asks about the
facts code cannot reveal, writes the harness and gate configuration when none
exists, and otherwise re-runs the existing harness when evidence goes stale.
The decision is recorded in [`docs/method.md`](../docs/method.md), where the
L3 contract is defined.

**Whether a gate is required to name a runnable harness.** That is a floor under
L3, and the shipped L3 proof-budget work owns the layer's existing floors.
Landing half of it here would split one rule across two plans, which is the
failure this plan is itself about.

**Whether the doctor step earns its separation.** pstack splits readiness from
driving. Our single worked example folds them together and has not wanted the
split, and one example is not enough to decide it.

## Non-goals

No browser driver, no sandbox, no seed data. The harness is whatever the
product's real entry point needs, and a tool that ships an opinion about that
is a tool for one stack.

L3's five clauses are not touched. This adds the missing producer in front of
them; it does not soften anything they check.

The generator does not get to write the manifest's `actor`. That field records
who ran the thing, and a generator that fills it in is a generator that credits
runs to itself.

## Acceptance criteria

- [x] The subcommand-or-skill decision above is written into `docs/rules.md` or
      `docs/method.md` before anything ships
      (proof: unspecified:the product-surface decision is recorded in
      docs/method.md and requires human review rather than an executable check)
- [x] The generator, run against this repository, produces a harness that emits
      a report carrying all four `adoption` assertions
      (proof: test:.software-factory/evidence/adoption-scenario.sh)
- [x] That generated harness replaces the hand-written
      `.software-factory/evidence/adoption-scenario.sh`, and its path sits
      inside the `adoption` gate's activation paths so editing it expires the
      evidence
      (proof: test:.software-factory/evidence/adoption-scenario.sh)
- [x] `sf check --rule L3.GATE_HAS_FRESH_EVIDENCE` is green on the manifest
      sealed against the generated harness's report, and goes red when
      `src/init.rs` changes without the harness being re-run
      (proof: test:.software-factory/evidence/adoption-scenario.sh; a scratch
      copy mutated src/init.rs and produced the expected stale-evidence finding)
- [ ] The generator runs against a repository nobody tuned it for whose product
      has a real startup dependency, and the harness it writes reports failed
      when that product is broken
      (proof: deferred:no corpus with a startup dependency is chosen yet)
- [ ] An adopter enabling L3 reaches a first sealed gate without hand-writing
      the policy entry, the manifest, the report and the harness
      (proof: deferred:`sf init` writes `gates: {}` and the interview has no
      answer that enables L3)
- [ ] If a check ships with this, `sf verify` proves it fires on its own
      mutation fixture
      (proof: deferred:whether any check ships is undecided above)

**Exit condition:** this repository's `adoption` evidence is produced by a
harness the tool generated rather than one we wrote, the gate is sealed against
that report and `sf check` is green, and the same generator run against a
repository nobody tuned it for produces a harness whose report goes red when
that product is broken.
