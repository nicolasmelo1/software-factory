---
name: factory-author
description: Turn a requirement into machine-checkable policy for the software factory — gates, activation paths, required assertions, and new catalog rules. Use when starting a phase of work that needs a completion gate, when a convention keeps being re-explained in review, or when someone asks "how do we stop this from happening again".
---

# factory-author

Your output is **policy and prose**, never a promise to remember something. If
the answer to "how do we stop this from happening again" is a sentence in a
document with no check behind it, you have not finished.

## When a convention keeps being re-explained

That is a rule. Write it.

1. `sf catalog` — check it does not already exist.
2. Write `.software-factory/rules/<name>.yaml`. `why` and `fix` are mandatory;
   the catalog will not load without them. Write `why` for the person who will
   want to delete this rule in a year: state what goes wrong without it, not
   what it does.
3. Write the smallest repository that violates it under
   `.software-factory/mutations/<RULE_ID>/`, with its own tiny
   `.software-factory/policy.yaml` enabling only that rule.
4. `sf fixtures`, then `sf verify --rule <RULE_ID>`. **If it does not fire, the
   rule is broken — fix the rule, not the fixture.** A tree-sitter query that
   matches nothing is the easiest bug to ship here and it looks identical to a
   rule that works. If the rule declares queries for several languages, the
   fixture needs a file in each: `verify` rejects a rule proven in only some of
   them, because three broken queries hide behind one that works.
5. Add the rule's section to the repository's rules document.
6. If the repository already violates it: `sf ratchet --months 6`. Say in the
   pull request how many violations were frozen and when they come due.

Choosing a layer: **L0** where things live, **L1** how code reads, **L2** a
derived artifact must not drift from its source, **L3** a real actor must
achieve an effect, **L4** docs and plans, **L5** the guardrail itself, **L6** a
class of defect worth hunting.

For L6, prefer wiring an existing tool over writing a rule: a `toolchain` rule
that asserts the scanner still runs is worth more than a bespoke check that
reimplements a fraction of it. Write a structural L6 rule only for a hazard no
tool covers — the concurrency shapes are the example, because no checker can
decide deadlock and the shapes that cause it are the decidable part.

## When a phase of work needs a completion gate

A gate answers: *what would make this actually done?* Not "the tests pass" —
what a customer would observe.

```yaml
gates:
  <name>:
    activation: ["src/checkout/**", "src/payments/**"]
    evidence: "evidence/<name>.json"
```

- **Activation paths are the implementation, not the tests.** They are what
  turns the gate on, so they must be the files that cannot change without the
  claim needing to be re-proven.
- **Required assertions must be observations, not claims.** `order.created`
  read back from the API is an observation; "the agent said it worked" is not.
  Prefer assertions on state after the fact over anything the actor reports.
- **Name at least one negative path** — idempotency, authorization, a duplicate
  submission, a refused payment. A gate with only the happy path proves the
  demo, not the product.

Write the manifest skeleton with the runs and required assertions, and leave
the digests empty. `factory-evidence` fills them.

## When the work needs a plan

Write it as a declaration: YAML frontmatter, a title, at most a few lines of
why. `L4.PLAN_IS_A_LINKED_DECLARATION` holds the shape.

```yaml
---
sources:   [{ id: S1, kind: decision, who: ..., said: "..." }]
touches:   ["src/checkout/**"]          # files it changes
untouched: ["src/payments/**"]          # only when one looks like it must change
criteria:  [{ id: A1, statement: "...", proof: "test:PATH", sources: [S1] }]
crosses:   [{ mechanism: ledger, answer: works, proof: "test:PATH" }]  # or refused+by, n/a+why
guarantee: [{ holds: "...", not_against: "..." }]
failures:  [{ step: "...", when: "...", goes_to: "..." }]
cut:       ["... : refused at boot"]
exit:      "the externally visible effect that ends it"
---
```

- **Every criterion names a source, every source is named.** An orphan on
  either side changes nothing that is built: cut it.
- **Crossings are derived.** With `mechanisms:` set, every mechanism whose
  `paths` a `touches` entry reaches must be answered.
- **Prefer refused to supported.** A composition the work does not need is a
  `cut` line and one check at boot, not a fix found in review.
- **Delete any line whose absence would change nothing** the builder or the
  reviewer does.
- **Facts about the code are pinned, not trusted.** A source cites a file in
  `ref:` and names what it means in `symbol:`, never a line number. Before work
  on a plan starts, run `sf ready <plan>`: it reports what no longer exists and
  what changed since the plan's `base:`. Fix or re-read those lines, then
  `sf ready <plan> --stamp` writes `base:` at HEAD. It is not a pull request
  gate: a plan may drift until somebody is about to build it.

## The line you do not cross

You propose policy. **A human merges it.** Never disable a rule, widen a glob,
or extend a `review_by` to make a red build green — that is indistinguishable
from a fix at the diff level, and it is the exact failure this whole thing
exists to catch. Report the finding and say what you think the right call is.
