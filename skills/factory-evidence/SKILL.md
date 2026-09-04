---
name: factory-evidence
description: Create or run the proof behind a software-factory L3 gate and seal evidence that survives re-verification. Use when a feature needs its first gate, when a gate is red for missing or stale evidence, or when asked to prove a change actually works end to end.
---

# factory-evidence

A feature needs proof. Your job is to make its gate green **by proving the
thing**, never by making a manifest say so.

First decide which state you are in:

- **A runnable harness exists:** re-run it, collect fresh observations and
  seal its evidence.
- **The feature has no harness or gate:** discover the customer flow, write
  the harness and gate, run the first proof, then seal it.

`sf` re-reads the raw report, recomputes its digest, and re-checks every
required assertion. Nothing you write in the manifest survives contact with a
report that does not back it.

## When the harness does not exist yet

Read the repository before asking questions. Find its README, local-development
commands, service manifests, seed data, test helpers, health endpoints and any
existing end-to-end tests. Run its doctor or readiness command when one exists.
Do not infer that `npm test` or `cargo test` starts the product: tests are not a
customer flow.

Settle only the facts code cannot reveal with the maintainer:

1. Which actor meets this flow, and what outcome can that actor observe?
2. What exact command starts the real entry point, and what dependencies,
   credentials or seed state does it require?
3. What are the smallest observable assertions that make the outcome true?
4. How can the harness create and clean up test data without borrowing an
   internal shortcut the actor would not have?

If an answer is missing, stop and record the prerequisite. A harness that
guesses a startup command is a runbook pretending to be a proof.

Create `.software-factory/evidence/<gate>-scenario.sh` (or the repository's
equivalent executable location). It accepts explicit inputs, starts the real
entry point, waits for readiness, drives the public interface, collects the
observations, and cleans up processes and data even when interrupted. Shell is
a good default for orchestration; use the repository's normal language when
the observations need its client libraries. Do not add a browser driver, a new
test framework or a mock simply to make this easier.

The program emits a JSON report with a customer-worded goal and non-empty,
stable assertion types:

```json
{
  "scenario": "checkout",
  "status": "passed",
  "goal": "As a shopper, place an order and see its confirmed total.",
  "assertions": [
    {"type": "order.created", "status": "passed"},
    {"type": "order.total_visible", "status": "passed"}
  ]
}
```

Create or extend `gates.<gate>` in `.software-factory/policy.yaml`. Its
`activation` includes the feature's implementation **and the harness itself**;
its `evidence`, `plan`, and `required_assertions` point at the manifest, the
plan, and every assertion that plan requires. The harness is not the actor:
the person or agent actually invoking it belongs in the evidence manifest.

## The proof loop

1. `sf check --rule L3.GATE_HAS_FRESH_EVIDENCE` — read which gate and why.
   `missing` means never proven; `stale` means the implementation moved since
   it was.
2. Read the gate's `required_assertions` in the manifest. Those are the claims
   you must produce observations for.
3. **Run the real thing.** Start the actual entry point a customer would use
   and drive it through the harness. Record the observations in its report.

4. `sf seal <gate>` — recomputes the implementation digest and every report
   digest from disk.
5. `sf check --rule L3.GATE_HAS_FRESH_EVIDENCE` to confirm.

Then make a harmless scratch-copy change to one activation path and confirm
the same check goes red. A proof that survives an implementation change is not
attached to the product.

## Rules that will catch you

- **An assertion you could not evaluate is `unsupported`, and `unsupported` is
  not `passed`.** Record it honestly. `sf` fails the gate either way, and the
  finding is the point.
- **The goal must read like a customer, not a recipe.** No absolute paths, no
  source directories, no fixture paths, no expected command transcript. A goal
  that hands the actor the answer proves the actor can follow instructions.
- **Never hand-edit a digest.** `seal` recomputes from disk; a digest you typed
  is a claim about a file you did not read.
- **`seal` cannot launder a failure.** It recomputes digests. A report with a
  failing assertion stays a failing report.
- **Stale means re-run, not re-seal.** If the implementation changed, the old
  run did not exercise the new code. Sealing without re-running is the single
  most tempting move here and it is a lie with a hash on it.

## If it cannot pass

Stop and say so. A gate that will not go green is usually reporting a real
product defect, and the defect is worth more than the green build. Report what
failed, what you observed, and what you think it means.
