---
name: factory-triage
description: Read a software-factory report, explain what actually broke, and fix it. Use when `sf check` or `sf verify` is red, when CI fails on a factory rule, or when someone asks whether a rule is worth keeping.
---

# factory-triage

## Read the report first

`sf check` already prints, for every finding, what the rule requires, why it
exists, and how to fix it. Read that before touching anything. `sf explain
<RULE>` gives the same for one rule.

`sf check --format json` when you need to work through many findings
programmatically.

## Triage order

**`sf verify` failures come first.** A rule that no longer fires on its own
mutation means every green run since it broke proved nothing. Fix the rule
until the fixture trips it again. Never adjust the fixture to match a broken
rule — the fixture is the specification.

**Then findings, by severity.** `critical` is L2/L3/L5: drift, a weakened
guardrail, an unproven effect, or a rule nothing proves. Those are never style.

**`L5.NO_INERT_RULE` is not a nuisance.** It means a rule is switched on and
looking at nothing — it has been passing every run and reading like protection.
Either point it at something or switch it off and write down why in the rules
document. Do not leave it enabled and inert.

One shape of it arrives without anybody editing the policy: a rule carrying a
`when` that names a dependency version, after somebody moved the pin. The
finding names the range the rule was written for and the version the manifest
declares now. There is nothing to configure — repoint the rule at the version
this repository installs, or delete it along with the version it described.
Editing the manifest back is not a resolution, and neither is dropping the
`when`, which leaves the rule firing on code that is now right.

## The four honest resolutions

1. **Fix the code.** The default. The `fix` line tells you the shape.
2. **Fix the rule.** If the rule is genuinely wrong — a query that matches
   something it should not, a glob that is too broad — change it deliberately,
   update its `why`, and re-run `sf verify`. Say in the pull request that the
   rule changed and why. That is the reviewable act.
3. **Freeze it, if you are adopting the rule today.** `sf ratchet --months 6`,
   and say how many violations were frozen and when they come due. This is only
   honest at adoption time, not as a way to get past a red build.
4. **Say you cannot.** Report the finding, explain what it would take. A
   blocked build is information.

## What is not a resolution

- Widening a glob or a scope so the finding stops matching.
- Adding a ratchet key by hand for a violation you just wrote.
- Pushing a `review_by` date out without touching the underlying debt.
- Disabling the rule in policy.
- Suppressing at the source (`# noqa`, `@ts-ignore`, `#[allow]`) — and
  `L1.NO_BLANKET_SUPPRESSION` will catch the blanket form anyway.

Each of these is indistinguishable from a fix in the diff and destroys the
thing the rule was protecting. Most of them are now caught mechanically —
`L2.FACTORY_CONFIG_IS_LOCKED` sees the guardrail file change and
`L2.POLICY_ONLY_TIGHTENS` sees which direction it went — but do not treat
"the checker did not catch it" as permission. If one of these is genuinely
right, it is a human's call: say which one you would pick, and why, and stop.

## When someone asks whether a rule is worth keeping

Good question to ask. Answer it from the rule's `why`, not from how often it
fires. A rule that never fires may be the reason the problem stopped happening.
A rule whose `why` no longer describes a risk this repository runs is a rule to
retire — deliberately, disabling it in policy and removing its prose section in
the same commit.
