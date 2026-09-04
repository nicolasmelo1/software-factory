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

## Why seven layers

The numbers are identity, not sequence. Six of them are about the shape and
provenance of the code; the seventh is a different question — which classes of
failure this repository is actively hunting, and whether the hunt still runs.

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

### L2 — Contract: no drift from the source of truth, including the guardrail's

Every rule in this catalog is enforced by a file an agent can edit, which makes
the guardrail itself a source of truth with derivatives. So it is locked like
any other: `L2.FACTORY_CONFIG_IS_LOCKED` covers the policy, the ratchet, the
local rules, the root allowlist, the workflow and the hooks.

The lock does not make the edit impossible — `sf lock` regenerates it, and an
agent could run that. It makes the edit *undeniable*: a second deliberate line
in the diff, on a path a code owner watches. That is the honest claim, and it
is the same property the rest of L2 has.

`L2.POLICY_ONLY_TIGHTENS` goes further and reads the direction of the change.
Disabling a rule, widening an exclusion, narrowing a scope, raising a ceiling,
freezing a new violation on an already-enabled rule, deferring a review date —
each of those is what an agent does when a check stands between it and a green
build, each costs nothing to write, and each is invisible six months later. A
newly enabled rule may seed the debt it first exposes; that is its adoption
baseline, while later additions remain a weakening. Strengthening passes
silently, so the rule never taxes the direction you want.

The rest of L2:

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

**A runnable harness is discovered by a skill, not guessed by a subcommand.**
`factory-evidence` reads the repository's actual startup path, asks for the
facts code cannot reveal, and writes the small program that drives one
customer-visible flow and emits the gate report when no harness exists. When
one does exist, it re-runs it rather than inventing a replacement under
pressure to make a gate green. The harness is an activation path of the gate it
serves, so changing either the product or the way it is driven expires the
evidence.

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

The same attachment applies to the guardrail's own prose. A rule's `fix` is
read at the one moment somebody is trying to comply, so
`L4.RULE_PROSE_NAMES_A_REAL_COMMAND` compares every command that prose quotes
with the command-line definition itself: a fix that sends the reader to a
subcommand this binary never had makes the whole rule look wrong.

A promise on a page is the same failure one level out. A README states an
effect, nothing joins the sentence to the run that proved it, and the sentence
ages into false while reading exactly as it did on the day it was true.
`L4.CLAIM_CITES_ITS_EVIDENCE` asks for the join and nothing else:

```
<!-- claim: IMPORT_50K_UNDER_60S proven-by: bulk-import -->
Import fifty thousand rows in under a minute.
```

`proven-by` names a gate, the gate carries evidence, and L3 already expires
that evidence when the implementation digest behind it moves, so the promise
goes red *through* the gate instead of through a second copy of that logic.
What no check can do is notice a promise nobody marked. That limit is written
into the rule's `why` rather than left to be discovered.

### L5 — Meta: the guardrail is proven to fire, and is pointed at something

Two failure modes, not one.

Without it every layer above is theatre. A check with a broken query or an
empty scope passes silently and looks exactly like a check that works.

Every rule ships with the smallest repository that violates it, and `sf verify`
fails if the rule does not fire there. Where a rule carries a query per
language, every one of those languages must be shown tripping it — otherwise
three broken queries hide behind the one that works. The pre-commit hook and CI
both run `verify` before `check`, because a check that stopped firing is the
cheaper failure to discover.

But firing on a fixture says nothing about whether the rule is pointed at
anything *here*. An enabled lock with an empty scope, or a hazard rule with no
tools declared, passes every run forever and appears in every report as a rule
that found nothing — which reads exactly like a rule that is protecting you.
`L5.NO_INERT_RULE` refuses that state: configure the rule, or switch it off and
write down why. This one is not hypothetical. It was written after this tool's
own scaffolding wrote an empty scope over a critical lock, and nothing noticed
until the lock file failed to appear.

### L6 — Hazard: the defect classes this repository hunts

The other layers are about whether code has the right shape and provenance.
This one is about whether anything is looking for the failures that shape does
not prevent.

It deliberately owns no scanner. Vulnerability databases, secret detectors,
static security analysers and race detectors already exist, they are far better
than anything that could live here, and they are different per language. What
is missing in most repositories is not the tool — it is the guarantee that the
tool is still wired in. A scanner someone removed to make CI faster fails
exactly like one that was never added, which is the point.

So a rule names a concern and the check asserts something covering it actually
runs. The concern is language-neutral; the tool is the per-language adapter —
the same split as L0's grammar and query.

**And then there is the part that has to be said plainly: no checker decides
whether a program deadlocks.** It is undecidable in general. A tool claiming
otherwise is worse than one that stays silent, because it teaches you to trust
it. What is decidable is the *shape* that produces the deadlocks and starvation
people actually ship — blocking while holding a lock, awaiting while holding a
synchronous one, taking a second lock inside the first — and those are
structural, so they are checkable. `L6.ONE_LOCK_AT_A_TIME` cannot tell a
correctly ordered pair from a dangerous one, because ordering is a global
property and the rule sees one function. It makes the second acquisition
visible, which is the part that is otherwise invisible, and pushes the ordering
into the ratchet where somebody has to write it down.

Data races get the same treatment from the other direction: nothing static
finds them, so the rule requires the dynamic detector to run.

## The rule about who merges

An agent may propose policy. A human merges it.

Everything here is designed so an agent can do the work — author the rule, seed
the ratchet, run the proof, seal the evidence. None of it is designed so an
agent can decide that the rule no longer applies. That single boundary is what
separates a factory from a system grading its own homework, and no amount of
tooling substitutes for it.
