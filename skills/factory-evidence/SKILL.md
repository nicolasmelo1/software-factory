---
name: factory-evidence
description: Run the proof behind a software-factory L3 gate and seal an evidence manifest that survives re-verification. Use when a gate is red for missing or stale evidence, when closing a phase that has a completion gate, or when asked to prove a change actually works end to end.
---

# factory-evidence

A gate is red. Your job is to make it green **by proving the thing**, never by
making the manifest say so.

`sf` re-reads the raw report, recomputes its digest, and re-checks every
required assertion. Nothing you write in the manifest survives contact with a
report that does not back it.

## The loop

1. `sf check --rule L3.GATE_HAS_FRESH_EVIDENCE` — read which gate and why.
   `missing` means never proven; `stale` means the implementation moved since
   it was.
2. Read the gate's `required_assertions` in the manifest. Those are the claims
   you must produce observations for.
3. **Run the real thing.** Start the actual entry point a customer would use.
   Drive it the way they would. If your harness produces a JSON report, point
   the manifest at it; if not, write one:

```json
{
  "scenario": "buy-and-refund",
  "status": "passed",
  "goal": "Buy a bike helmet and then refund it, using only the storefront.",
  "assertions": [
    {"type": "order.created",  "status": "passed"},
    {"type": "refund.settled", "status": "passed"}
  ]
}
```

4. `sf seal <gate>` — recomputes the implementation digest and every report
   digest from disk.
5. `sf check --rule L3.GATE_HAS_FRESH_EVIDENCE` to confirm.

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
