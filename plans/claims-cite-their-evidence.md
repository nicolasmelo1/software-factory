# Claims cite the evidence that proves them

A landing page and a README make promises. Nothing here joins a promise to the
thing that proved it, so the two come apart the ordinary way: the promise was
true when it was written, the code moved, and the sentence stayed.

The reference half of documentation should not be checked at all, it should be
generated. `docs/rules.md` is generated from `catalog/`, and
`L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE` runs `sf docs` and diffs the result,
so it cannot sit out of date with the rules it describes. Every SDK reference
page should be built that way, and then this rule has nothing to say about it.

This plan is about the other half: prose with no source to be generated from.
A landing page, a "what this does" section, a benchmark number in a README.

## The primitive already exists

`L4.EVERY_RULE_HAS_A_WHY` is a bidirectional citation check. Every enabled rule
must be cited in prose, and every rule id appearing in prose must exist in the
catalog. Its marker is a configurable regex and its scope is a glob
([`src/checks/cadence.rs`](../src/checks/cadence.rs)). The only hard-wired part
is what a citation resolves *against*, which is the catalog.

Point the same engine at gates:

```
<!-- claim: IMPORT_50K_UNDER_60S proven-by: bulk-import -->
Import fifty thousand rows in under a minute.
```

`proven-by` names a key in `gates:` in the policy. A claim naming a gate that
is not there fails. A gate that is there carries evidence, and
`L3.GATE_HAS_FRESH_EVIDENCE` already fails when the implementation digest of
the activation paths has moved, so the promise goes red *through* the gate.
Writing a second freshness check here would be a worse copy of the one that
already works.

The rule is not named here on purpose. `L4.EVERY_RULE_HAS_A_WHY` resolves rule
ids in every markdown file, this one included, so a plan that picks an id for a
rule it has not written yet fails the check that plan is about to extend. The
id is chosen in the commit that writes the rule.

## Three decisions

**Two directions, not three.** Every claim names a proof, and every named gate
exists. The third direction, every gate is claimed somewhere, is wrong: most
gates have nothing to do with a marketing page.

**A claim id, not just a gate reference.** The id is what survives an edit of
the sentence. Without it, moving a promise into another paragraph reads as one
claim deleted and another added, and the reviewer loses the thread of which
promise was ever proven.

**The marker is a comment, not syntax.** It has to work in markdown, MDX and
HTML without a build step, and it has to be invisible in the rendered page.

## What this does not do

It cannot enumerate the promises on a page, so it cannot notice a new unmarked
one. That limit belongs in the rule's `why`, stated, not implied.

It also does not verify that the sentence means what the gate proved. Only a
reader does that, human or model, and it happens once, when the promise is
written. What CI gets for free is the join: the proof exists, it is enabled,
and it is not certifying code that has since changed. That is where the drift
actually lives, because a promise is rarely written false, it ages into false.

**Exit condition:** a claim marker in a scoped page naming a gate that is not
in the policy fails `sf check`, and a claim whose gate lost its evidence to a
code change fails through `L3.GATE_HAS_FRESH_EVIDENCE`, both proven by mutation
fixtures.

## Acceptance criteria

- [ ] A cadence mode `claim_citations` resolves claim markers against `gates:`
      and fails on a gate the policy does not declare
      (proof: deferred:the mutation fixture for the new L4 rule, written in the
      same commit as the rule)
- [ ] A claim marker that parses but names no gate fails the same way a
      criterion with no proof marker does
      (proof: deferred:same fixture as above)
- [ ] Changing the code behind a claimed gate turns the page red with no
      freshness logic added to this mode
      (proof: test:.software-factory/mutations/L3.GATE_HAS_FRESH_EVIDENCE/)
- [ ] An unmarked promise is not detected, and the rule's `why` says so rather
      than leaving it to be discovered
      (proof: unspecified:no check can enumerate the promises on a page)
