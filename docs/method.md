# The method

The observation this is built on: a program is the surface of a much larger
object. The specification, the derivation, the tradeoff, the correctness
argument, the product judgement — none of it is needed to *run*, so all of it
gets stripped away. What remains is the executable shadow.

For most of software history that was survivable, because senior engineers
carried the missing object in their heads. It does not survive delegation. An
agent asked to extend a system from a fraction of the context that created it
produces work that is syntactically correct and semantically incomplete: it
looks like progress and requires a human to audit it before anything can be
trusted.

The response is not more context. It is context with a shape — rules that are
executable at the moment of work, and proof obligations that are checkable
before a human sees the result.

## One sentence

**Every rule that matters is written twice — once as prose that says why, once
as a check that fails. And every check has a mutation that proves it fires.**

The prose without the check is advice, and it drifts the first time someone
ignores it. The check without the prose is a wall an agent hits with no way to
tell whether it is protecting something or is merely old. Neither half works
alone, which is why `L4.EVERY_RULE_HAS_A_WHY` enforces the pairing in both
directions: an enabled rule that nothing explains fails, and a cited rule id
that does not exist fails.

## Why six layers

They are ordered by what they cost to adopt and by what they need to already be
true, not by importance.

### L0 — Shape: where things live

Error types in one module per domain. Data access behind a repository. One
entrypoint per file. Internal modules not imported across the boundary.

These are the rules a senior engineer applies without noticing. They resist
being written down because they feel obvious — right up until a swarm of agents
each makes a locally reasonable choice and the shape dissolves. Placement is
cheap to enforce and expensive to recover.

L0 is the layer most likely to be adopted too early. A shape you have seen
twice is a coincidence; wait for the third.

### L1 — Grain: how the code reads

A complexity ceiling, a ban on the untyped escape hatch, and no suppression
without a named code and a written reason.

The ceiling is not a style preference — it is the cheapest language-neutral
proxy for "a reviewer can hold this in their head", and agents are structurally
prone to growing a working function by one more branch, because appending a
branch is a smaller diff than a refactor.

The escape-hatch ban matters most at the point of *introduction*. Banning
`typing.Any` at the import is what kills `dict[str, Any]`, `-> Any` and bare
`x: Any` in one rule, instead of only the annotations a checker can see.

And every message names the alternative. The lint failure is the documentation
an agent actually reads; a ban with no replacement just teaches it to suppress.

### L2 — Contract: no drift from the source of truth

The general shape is not "OpenAPI versus SDK". It is: *one thing is the truth,
and everything derived from it must be provably derived.* A database schema and
its migrations and its types. A plan catalogue and its billing engine and its
pricing page. Design tokens and the stylesheet.

Three primitives carry it. A **hash lock** makes hand-editing a derived
artifact impossible rather than merely discouraged. A **dated exception**
(`review_by`) stops a temporary allowance from becoming the permanent shape of
the codebase. And **hierarchical exit codes** let a caller tell "the tool could
not run" from "the repository has violations".

### L3 — Effect: a real actor achieved the outcome

The gate is not "the tests pass". It is: *an actor shaped like the customer
achieved the observable effect, judged from state rather than from the actor's
own prose.*

Five properties make it real, and each one closes a specific way a green gate
comes to prove nothing:

1. **Activation from touched paths.** No label to forget, no pull-request
   sentence to route around.
2. **The manifest is re-verified, not trusted.** The raw report is re-read and
   re-hashed, because a summary can assert a pass the report never contained.
3. **`unsupported` is not `passed`.** An assertion the harness could not
   evaluate is the most common thing counted as a success.
4. **Evidence expires with the implementation.** A digest of the activation
   paths is recorded, so changing the code kills the evidence instead of
   letting it certify something it never saw.
5. **The goal is checked for leaked answers.** A goal that names the source
   tree, a fixture directory or an absolute path is a replay recipe, not a
   customer asking for something.

Whether the actor is an agent, a browser driver or a person depends on who your
customer is. The rule is the same.

### L4 — Cadence: docs, plans and rules stay attached

Three separations, each of which fails a specific way when it collapses:

- **Documentation is a stateless snapshot** of the current shape. It describes
  shipped mechanics or durable structure. It never carries sequencing — the
  moment it does, it starts rotting on a schedule nobody is watching.
- **Plans carry the future**, one per unit of work, each declaring an exit
  condition that names an *externally visible effect*. Not a merge: the merge
  is precisely the thing you can produce without the effect ever happening.
- **One execution order.** A single ordered document, short on purpose, is what
  stops parallel agents from each choosing their own next priority.

### L5 — Meta: the guardrail is proven to fire

Without it every layer above is theatre. A check with a broken query or an
empty scope passes silently and looks exactly like a check that works.

Every rule ships with the smallest repository that violates it, and `sf verify`
fails if the rule does not fire there. The pre-commit hook and CI both run
`verify` before `check`, because a check that stopped firing is the cheaper
failure to discover.

## The rule about who merges

An agent may propose policy. A human merges it.

Everything here is designed so an agent can do the work — author the rule, seed
the ratchet, run the proof, seal the evidence. None of it is designed so an
agent can decide that the rule no longer applies. That single boundary is what
separates a factory from a system grading its own homework, and no amount of
tooling substitutes for it.
