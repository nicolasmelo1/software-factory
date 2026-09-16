# Siblings share the helper

Issue #54 contains two different defects under one observation. This plan owns
the first: sibling functions consume the same shape, most use the local helper
that encodes one decision, and one open-codes or omits it. The sibling plan,
[calls-to-one-capability-share-the-guard.md](calls-to-one-capability-share-the-guard.md),
owns guard agreement between callers. They share provenance, not an engine or
an exit condition.

A name-family heuristic was measured and found nothing. The current `forwarder`
engine demonstrates why captures must be joined semantically: it maps imported
and local names, then compares the function and callee (`src/checks/forwarder.rs:61-90`,
`:94-114`). Helper adoption needs a different relation — functions grouped by
what they consume, then a helper used by the group — and must not infer a group
from similar spelling.

## What changes

**A sibling-adoption check kind.** Per-language queries capture functions,
their consumed parameter type or common input field, and calls to local
helpers. The engine groups functions by that consumed shape, identifies a
helper adopted by a configurable minimum of the group, and reports a sibling
that performs the same role without that call. The finding names the group,
the adopting siblings and the outlier, so the maintainer can reject a bad
grouping rather than trust a score.

**Open-coding is evidence, not just absence.** A missing helper call is reported
only when the outlier reads the same input fields the helper resolves or
normalises. A function that happens to accept the same type but never touches
the decision's inputs is not a sibling for this rule. Wrappers and delegated
calls are followed only within the file; uncertainty stays silent and is
counted in the corpus report.

**The threshold is measured.** The fixture proves the fallback-id defect and a
legitimate sibling that does not need the helper. Before default enablement,
the proposed grouping and threshold run over the repository from issue #54;
the known outlier must appear, with no false positives, and the corpus report
records groups considered and groups skipped as uncertain. A fixture proves it
can fire; this measurement proves it can find code that exists.

## Acceptance criteria

- [ ] Functions are grouped by a shared consumed type or input field, never by
      a name prefix alone
      (proof: test:src/checks/sibling_adoption.rs)
- [ ] A sibling that reads the helper's decision inputs without calling the
      helper is reported with the adopting siblings and helper named
      (proof: test:src/checks/sibling_adoption.rs)
- [ ] A same-type function that does not consume those inputs and a group below
      the measured threshold stay quiet
      (proof: test:src/checks/sibling_adoption.rs)
- [ ] The mutation reproduces a delete handler that reads only the primary id,
      while its repair uses the fallback helper
      (proof: test:src/fixtures.rs)
- [ ] The measured corpus finds the known outlier, records every skipped
      uncertain group, and produces no false positives before enablement
      (proof: unspecified:the source repository and adjudication live outside
      this repository)

**Exit condition:** a measured sibling group cannot leave one handler
open-coding a decision the others take from a shared helper without `sf check`
naming the helper, the group and the outlier, and the rule has proved it fires
on real code rather than only on its own fixture.
