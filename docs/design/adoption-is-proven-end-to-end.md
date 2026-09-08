# Adoption is proven end to end

This repository ships L3 and did not run it.

`.software-factory/policy.yaml` carried `gates: {}` with both L3 rules switched
on. Neither could produce a finding under any change, and neither said so: they
appeared in every report as rules that found nothing, which is the exact state
`L5.NO_INERT_RULE` was written to refuse one layer down. The rule could not see
it, because `inert_reason` had no arm for `evidence` or for `gate_coverage`.

Closing that blind spot is the other half of this work. This plan is the half
that has to come with it, because a rule that now says "declare a gate" is only
honest if the repository that ships it declares one.

## What the gate is about

The tool's customer-visible effect is not `sf check` returning zero. It is a
person with an existing codebase adopting the factory and getting a build that
blocks their next bad change without burying them in the mess already there.
That is the promise the README quickstart makes, and it is the one thing here
that a person can meet.

So the scenario is adoption, and the activation paths are the code that
performs it: `src/init.rs`, `src/interview.rs`, `interview/decisions.yaml` and
`templates/`. Change how a repository gets set up and the evidence expires,
which is the correct outcome — the last run proved a scaffolding that no longer
exists.

## Why the fourth assertion is the load-bearing one

Three of the four are about the setup completing. The fourth is the only one
about the product working, and it is the one adoption actually turns on.

`sf init` freezes what it finds. On the corpus below that was 1,737 keys, and a
tool that handed an adopter 2,833 findings on day one would be uninstalled by
lunchtime. But a ratchet that freezes everything and then never fires again is
the same green build with extra steps. The effect worth proving is the pair:
green on the frozen baseline, red on the first new violation written after it.

Proving only the first half would be the failure this layer exists to catch.

## The corpus

`palmares`, at `cd135c71e1debc55c20e08059749fe37ee943e21` — a TypeScript
framework nobody tuned this tool for, 1,232 tracked files, copied to a scratch
checkout so the run could not touch the original. Not a fixture written to
violate: `sf init` selected its own default layers (L1, L4, L5), enabled 13
rules, and the 1,737 frozen keys are what it found in code written without any
of these rules in mind.

## Acceptance criteria

- [x] `sf init` on a repository that has never seen this tool writes a policy
      that loads, a rule document, a CI workflow and a pre-commit hook
      (proof: assertion:cli.init_scaffolds_a_policy)
- [x] `sf verify` is green immediately after that init, proving every rule it
      enabled fires on its own fixture
      (proof: assertion:cli.verify_is_green_after_init)
- [x] `sf check` is green on the baseline init froze, so adoption does not hand
      anybody a red build on day one
      (proof: assertion:cli.check_is_green_on_the_frozen_baseline)
- [x] One new violation written after adoption turns `sf check` red and names
      its file and line, so the ratchet froze the debt without disarming the
      rule
      (proof: assertion:cli.new_violation_fails_the_check)

**Exit condition:** `L3.GATE_HAS_FRESH_EVIDENCE` is active on this repository,
carries a sealed manifest whose report shows all four assertions passing against
a real checkout of a codebase nobody tuned this tool for, and goes red the next
time `src/init.rs` changes without the run being repeated.
