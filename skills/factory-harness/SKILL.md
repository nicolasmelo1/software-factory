---
name: factory-harness
description: Discover and generate the runnable harness behind a software-factory L3 gate. Use when a repository has a customer-visible scenario that needs its first gate, when a gate has no reproducible way to produce evidence, or when setting up end-to-end proof for a product.
---

# factory-harness

The repository needs a way to produce evidence, not another runbook that says
someone should do it. Your job is to leave behind a small program that starts
the real product, drives one customer-visible flow, records observations, and
writes the report an L3 gate verifies.

This is deliberately a skill rather than an `sf` subcommand. Starting a
product is a fact about its repository, credentials, dependencies and entry
point; a generic binary cannot discover it honestly. Do that discovery once,
write it down as executable code, and let `sf` verify the resulting evidence
thereafter.

## Start with the product, not the gate

Read the repository before asking questions. Find its README, local-development
commands, service manifests, seed data, test helpers, health endpoints and any
existing end-to-end tests. Run its doctor or readiness command when one exists.
Do not infer that `npm test` or `cargo test` starts the product: tests are not a
customer flow.

Then settle these facts with the maintainer. Ask only what the code cannot tell
you:

1. Which actor meets this flow, and what outcome can that actor observe?
2. What exact command starts the real entry point, and what dependencies,
   credentials or seed state does it require?
3. What are the smallest observable assertions that make the outcome true?
4. How can the harness create and clean up its data without borrowing an
   internal shortcut that the actor would not have?

If any answer is missing, stop and record the missing prerequisite. A harness
that guesses a startup command is a runbook pretending to be a proof.

## Write the harness

Create `.software-factory/evidence/<gate>-scenario.sh` (or the repository's
equivalent executable location). It must:

- accept explicit inputs needed to reach the product; never hide a credential
  in the script;
- start the product through its normal entry point and wait for readiness;
- exercise the public interface as the actor would — HTTP, browser, CLI or
  queue — rather than calling an internal function or querying a test-only
  database handle;
- collect the observations that establish each assertion;
- clean up processes and data on success, failure and interruption; and
- emit one JSON report containing `scenario`, `status`, a customer-worded
  `goal`, and non-empty `assertions` with stable `type` values and statuses.

The harness is not the actor. Its JSON report must not call the actor
`script`, `harness`, or another denied replay label. The person or agent who
actually invokes the harness belongs in the evidence manifest's `actor`.

Keep the harness boring and inspectable. Shell is a good default when it only
orchestrates existing commands; use the repository's normal language when the
observations require its client libraries. Never add a browser driver, a new
test framework or a mock solely to make the harness easier to write.

Use this report shape:

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

`unsupported` is not a pass. If an observation cannot be made, preserve that
status, explain the blocker, and stop; do not replace it with a guess.

## Connect it to the gate

Create or extend `gates.<gate>` in `.software-factory/policy.yaml` with:

- `activation` paths for the implementation the scenario certifies **and** the
  generated harness itself;
- `evidence` pointing at the manifest;
- `plan` pointing at the plan whose criteria this proves; and
- `required_assertions` listing every assertion type the plan requires.

Write the manifest and report paths before the first run, but never fill in a
digest by hand. Run the harness for real, write the report it observed, then:

```sh
sf seal <gate>
sf check --rule L3.GATE_HAS_FRESH_EVIDENCE
```

The check must be green. Then make a harmless scratch-copy change to one
activation path and confirm the same check goes red: a proof that survives an
implementation change is not attached to the product.

Finally, explain to the maintainer what the harness proves, where its inputs
come from, and the one command needed to re-run it. When an existing gate is
merely stale, use `factory-evidence`; this skill is for creating or replacing
the producer of evidence.
