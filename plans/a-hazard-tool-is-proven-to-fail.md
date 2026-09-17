# A hazard tool is proven to fail

L6 owns no scanner on purpose. A rule names a concern, and the check asserts
that something covering it actually runs — the tool is the per-language
adapter, and the concern is language-neutral. That split is right and this plan
does not touch it.

What the split leaves unasked is whether the thing being delegated to can
return non-zero.

## The finding this came out of

`L6.PERFORMANCE_REGRESSION_IS_GUARDED` is enabled in this repository and in the
first product it was adopted by. That product does exactly what the rule's
`fix` prescribes: `vitest bench` with a committed baseline, run in CI.

Point that command at a baseline claiming twenty times the throughput and it
prints the arrow and exits zero:

```
· adds up a five hour window of a busy week   [0.05x] ⇓
· decides against both windows                [0.05x] ⇓
exit 0
```

So the step cannot fail, and the rule's own `why` is the sentence it breaks:
*a committed baseline is what turns that from an incident into a failed build.*

Nothing in the catalog is wrong about the concern. The `fix` recommends a
command that reports rather than one that judges, and no check can tell the
difference, because the shape of the finding is "a tool is named" and the shape
of the risk is "the tool cannot say no".

## Why L5 cannot see it

`L5.NO_INERT_RULE` refuses a rule that cannot produce a finding — an empty
scope, a hazard with no tools declared. It reads the rule's own configuration.

The `toolchain` check reads the workflow and the manifests and looks for the
tool's name. Both are satisfied here, honestly and completely. Neither is in a
position to know that the process the name refers to exits zero on the defect
it exists to catch.

That is the L5 idea one level out, and it has the same failure signature the
layer was written for: it passes every run forever and appears in every report
as a check that found nothing, which reads exactly like a check that is
protecting you.

## What changes

- A hazard rule's declared tool carries a **falsifier**: the smallest
  repository with the defect planted, the command, and the expectation that the
  command exits non-zero on it and zero without it. It is the same object
  `sf verify` already builds for the catalog's own rules, pointed at the
  delegate instead of at the query.
- `sf verify` runs it. Not `sf check`: the tools are slow, some of them reach
  the network, and `verify` is already the pass whose job is proving the
  guardrail works rather than proving the repository is clean. The command
  machinery exists (`src/checks/command.rs`), including the flag that gates
  running anything at all.
- A hazard whose defect cannot be planted where the run happens — a detector
  that needs hardware, a scanner that needs a paid feed — declares that, with a
  reason, and the declaration is the finding's honest form. Same answer the rest
  of this catalog gives.
- `L6.PERFORMANCE_REGRESSION_IS_GUARDED`'s `fix` stops naming a command that
  only reports. For TypeScript that means reading the comparison and exiting
  non-zero past a declared threshold — and a threshold is a number somebody can
  raise, so it belongs to `L2.POLICY_ONLY_TIGHTENS` like every other ceiling.

## The second half of a performance claim

A digest of the activation paths expires evidence when the implementation
moves. A number about speed also expires when the machine moves, and nothing
records which machine produced it: the baseline in the product above was
measured under a path that no longer exists on any developer's disk, and is
compared against numbers from whichever runner CI was given that morning.

A benchmark baseline therefore names the environment it was taken on, and a
comparison across two different ones is reported as unavailable rather than as
a result. Bend's benchmark tree does this the obvious way — one pinned file per
hardware, medians of three runs — and it is the missing field rather than a new
idea.

## What this does not claim

It does not claim the tool is good. A secret scanner that catches the planted
key may miss every real one, and a benchmark that goes red past a threshold
says nothing about whether the threshold was chosen well. The claim is exactly
one step: a tool that cannot fail on a defect somebody planted will not fail on
one somebody ships, and that is worth refusing on its own.

## Acceptance criteria

- [ ] A hazard rule declares a falsifier, and `sf verify` fails when the
      declared command exits zero against the planted defect
      (proof: test:src/checks/hazard.rs)
- [ ] The same falsifier is required to pass without the defect, so a command
      that always fails is refused too
      (proof: test:src/checks/hazard.rs)
- [ ] A hazard rule with no falsifier and no written reason is a finding, and
      one with a reason is not
      (proof: test:src/checks/hazard.rs)
- [ ] `sf verify` without `--allow-commands` reports the falsifier as unrun
      rather than as passed
      (proof: test:src/checks/command.rs)
- [ ] `L6.PERFORMANCE_REGRESSION_IS_GUARDED` names a command that exits
      non-zero on a planted slowdown, and its own falsifier proves it
      (proof: test:src/fixtures.rs)
- [ ] A benchmark baseline carries the environment it was measured on, and a
      comparison across two environments reports unavailable rather than a
      verdict
      (proof: unspecified:where the field lives depends on whether the baseline
      stays a tool's own format or becomes an evidence artifact, and that is the
      first thing the work has to decide)

**Exit condition:** a repository whose performance guard cannot fail is told so
by `sf verify`, with the command it ran and the exit code it got, and the same
run proves that every other hazard tool it declares goes red on a defect
planted under it — so "something is looking for this" stops meaning "something
is mentioned in the workflow".
