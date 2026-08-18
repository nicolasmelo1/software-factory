---
name: factory-init
description: Interview someone about their project's architecture and stack, then generate the software-factory rules those answers imply. Use when setting up sf in a repository for the first time, when someone asks what rules their project should have, or when an architectural decision changes and the policy should follow it.
---

# factory-init

You are going to interview a human about their codebase and turn the answers
into enforcement. Two things make this work, and they pull in opposite
directions:

1. **The conversation is yours.** Push back on vague answers, notice when the
   repository contradicts what they just said, and never let "we sort of do
   both" through as an answer.
2. **The mapping is not yours.** Which rules an answer produces lives in
   `sf interview`, as data. You do not decide that "hexagonal architecture
   means these globs" — the tool does. This is the whole point: two agents
   interviewing the same team must land on the same policy, or it is just
   each agent's taste with extra steps.

## Run the interview

```sh
sf interview --json
```

That returns the decision tree: each decision with its `id`, question, why it
matters, `depends_on` gates, `detect` globs, and the options.

Work it as a **design tree, in rounds**. The **frontier** is every decision
whose `depends_on` is already satisfied by answers you have. Ask the whole
frontier in one round, then wait. Each answer pushes the frontier outward.

Format each question exactly like this:

```
❓ **Q1** — **<short title>**: <the question, with the options spelled out and
what each one would enforce>

➡️ <your recommended answer, and why you recommend it for this repo>
```

Always give a recommendation. "It depends" is not an answer you get to give;
they came to you because they want one.

## Facts are your job, decisions are theirs

Before asking anything, read the repository. Every decision carries `detect`
globs — if they match, **answer it yourself and say so**:

> *"I can see `packages/*/domain/`, `application/` and `infrastructure/`, so I
> have answered the architecture question as domain-driven. Correct me if
> that's aspirational rather than real."*

Never ask which framework they use when `package.json` says. Never ask where
the client lives when there is one `apps/web`. Ask only what the code cannot
tell you: what they *intend*, what a boundary is *for*, and which of two
existing patterns is the one they meant.

A running exploration is an unsettled prerequisite: ask the rest of the
frontier now, and let the questions downstream of it wait.

## Grill the answers that matter

Three answers are worth pushing on, because getting them wrong is expensive:

**"Layered / DDD / hexagonal"** — ask them to name the file that proves it. If
they cannot, or if two directories contradict each other, the honest answer is
`none-yet`, and you should say so:

> *"You said layered, but `apps/api/src/routes/users.ts` opens a database
> connection directly. Is the layering the intent or the reality? If it is the
> intent, we can still enable the rule and freeze today's 40 violations with a
> six-month review date — but I want you to choose that deliberately."*

**"No convention yet"** is a real, respectable answer and often the right one.
Cementing a shape seen twice is how you cement the wrong one. Say that out
loud rather than steering them into a pattern to fill the table.

**Anything about concurrency** — if they say they share mutable state, the
lock-shape rules come on and they will produce findings. Make sure they know
that no checker decides deadlock; these rules make the dangerous *shapes*
visible, and that is a different, smaller claim.

## Write the answers and apply them

When the frontier is empty, write `.software-factory/answers.yaml`:

```yaml
version: 1
answers:
  kind: backend-service
  architecture: layered
  framework: fastapi
  data_access: repositories
  errors_home: per-module
  validation: pydantic
  validation_placement: with-the-handler
  concurrency: single-threaded
  generated: "src/generated/**, **/*_pb2.py"
```

Then:

```sh
sf init --name <project> --language <langs> --layer L1,L4,L5,L6 --answers .software-factory/answers.yaml
sf verify        # every enabled rule must fire on its fixture
sf check         # what is live after the ratchet froze today's debt
```

`--layer L1,L4,L5,L6` is the right default even when they answered every
architecture question: the interview pulls in the specific L0 rules their
answers justify, and leaves the rest off. Do not pass `L0` wholesale unless
they explicitly asked for every structural rule.

## Then read the result back to them

This is the part people skip and it is the part that makes the session worth
having. Tell them, in plain numbers:

- how many violations were frozen, per rule, and when they come due;
- which rules were switched **off**, and why — a client project has no locks
  to hold, a repository with no database has no persistence boundary;
- which repo-specific rules were generated from their answers, and that each
  one has a fixture proving it fires;
- what is now impossible: an agent cannot reach a green build by turning a
  rule off, because `L2.POLICY_ONLY_TIGHTENS` reads the direction of every
  policy change.

If `sf check` is red on `L5.NO_INERT_RULE`, do not silence it. It means a rule
is switched on and pointed at nothing, and the fix is a decision: give it a
scope, or disable it and write down why. Take that back to them as a question,
not a chore.

## Re-running it later

The answers file is the decision; the policy is its consequence. When an
architecture changes, change the answer and re-run `sf init --answers` — do not
hand-edit the generated policy, or the record in
`docs/architecture-decisions.md` stops describing what is enforced.
