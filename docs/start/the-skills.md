# The skills

Four agent skills for Claude Code, shipped inside the binary so they cannot
drift out of step with the `sf` they drive. Install them once:

```sh
sf skills             # asks: this repository, or every project
sf skills --project   # <root>/.claude/skills, no question
sf skills --user      # ~/.claude/skills, no question
```

**Invoke them by name.** `/factory-init`, `/factory-evidence`, `/factory-triage`
and so on: an agent
may pick one up from its description when the conversation matches, but that is
not something to rely on, and a skill that silently did not load looks exactly
like one that did and had nothing to say.

Their job is to **author policy and produce evidence**, never to remember rules;
that is what the binary is for.

One boundary runs through all four: **an agent proposes policy, a human merges
it.** That is the only thread separating a factory from a system grading its own
homework, and no amount of tooling substitutes for it.

<!-- sf:generated skills-index -->
| Skill | What it is for |
| :-- | :-- |
| `/factory-init` | Interview someone about their project's architecture and stack, then generate the software-factory rules those answers imply. |
| `/factory-author` | Turn a requirement into machine-checkable policy for the software factory — gates, activation paths, required assertions, and new catalog rules. |
| `/factory-evidence` | Create or run the proof behind a software-factory L3 gate and seal evidence that survives re-verification. |
| `/factory-triage` | Read a software-factory report, explain what actually broke, and fix it. |

<!-- sf:end skills-index -->

## `factory-init` — setting up, or when the architecture changes

The one you use first. It runs the interview above.

> **You:** `/factory-init set up software-factory in this repo`

It reads the codebase before asking anything, answers what the code can answer,
and asks in rounds — each question numbered, each with a recommendation:

> ❓ **Q2** — **Architecture**: I can see `packages/*/domain/`,
> `application/` and `infrastructure/`, so this looks domain-driven. But
> `apps/api/src/routes/users.ts` opens a database connection directly. Is the
> layering the intent or the reality?
>
> ➡️ I'd answer `ddd` and freeze today's 40 violations with a six-month review
> date, rather than `none-yet` — but I want you to pick that deliberately,
> because it is a commitment to fix them.

Then it applies the answers, and reads the result back to you in numbers: what
was frozen and when it comes due, which rules were switched **off** and why,
which repo-specific rules were generated. Re-run it whenever an architectural
decision changes.

## `factory-author` — when you hear yourself repeating a review comment

You are lead on a TypeScript monorepo. It is the third time this month you have
written *"don't import the db directly in a component, go through the API"*.

> **You:** `/factory-author` third PR this month where someone imports
> `@acme/db` inside `apps/web`. I want this to stop being my comment and
> become a check.

It does not say "good idea, I'll remember". It checks the rule does not already
exist (`sf catalog`), writes the YAML with a mandatory `why` — written for the
person who will want to delete this rule in a year — writes the smallest
repository that violates it, runs `sf verify --rule`, and **if it does not fire
it fixes the rule, not the fixture.** Then `sf ratchet` if there is existing
debt, and it tells you how many it froze and when they come due.

What you get is a pull request: a rule, a fixture, a section in the rules
document. You read it and merge it.

Its other trigger is opening a phase of work — *"I'm rewriting billing, I want a
completion gate"* — where it designs the activation paths and required
assertions, and insists the assertions be observations read back from the
system, not claims the actor makes about itself.

## `factory-evidence` — when a feature needs proof

> **You:** `/factory-evidence` create and prove the checkout gate for this
> service.

If no harness exists, it reads how the product starts, identifies the public
flow and observations, then writes the runnable harness and L3 gate. The
harness becomes an activation path, so editing it expires the evidence it
produced. If a harness already exists and its evidence is `stale`, it runs that
program again; it never re-seals an old run and calls it proof.

It runs the real thing: starts the app, drives it through the entry point a
customer would use, collects the observations, writes the report, and only then
seals. And it tells you plainly when an assertion did not pass:

> *"`refund.settled` came back `unsupported` — the harness could not evaluate
> it. That is not a pass. The finding is the product's behaviour, not the gate."*

If the gate cannot go green, it stops and says so. A gate that will not pass is
usually reporting a real defect, and the defect is worth more than the build.

## `factory-triage` — the daily one

> **You:** `/factory-triage` CI is red, 12 findings, sort it out.

It reads the report (which already carries each rule's reasoning) and works in a
specific order: **`sf verify` failures first** — a rule that stopped firing
means every green build since then proved nothing.

The part that earns its disk space is the list of what is **not** a resolution:
widening a glob, adding a ratchet key for a violation you just wrote, pushing a
`review_by` out, disabling the rule, suppressing at the source. In a diff, each
of those is indistinguishable from a fix. The skill is told to name which one it
would pick, say why, **and stop**.

It has four honest resolutions, and one of them is *fix the rule*. If the rule
is wrong, you change it deliberately, update its `why`, and re-run `verify`.
That happened while building this: the `Exit condition` marker did not accept
`**bold**`. The rule was wrong, not the document.

The safety net underneath is mechanical now — if it (or any agent) tries to
disable a rule to go green, `L2.POLICY_ONLY_TIGHTENS` catches it, and it
survives even if the agent runs `sf lock` to cover the trail.
