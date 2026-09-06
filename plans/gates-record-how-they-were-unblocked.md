# A gate records how it was unblocked

Every gate already says why it stopped. `sf check` prints the rule's `why` and
its `fix` on every finding, that prose is hand written, and
`L4.RULE_PROSE_NAMES_A_REAL_COMMAND` keeps it honest about the commands this
binary accepts.

It is also byte for byte identical on every run. That is the gap. A weak model
reads the fix, makes an edit, stays red, and gets the same bytes back. There is
no gradient, so it tries a variation of the same wrong idea until it runs out of
turns. The `fix` is general advice about a rule. What breaks a loop is knowing
what has already been tried in this working tree, and seeing one worked example
of the transformation that cleared the same rule somewhere nearby.

Three parts. None of them takes prose from a model, because the shortest path
from red to green in a repository like this one is to weaken the guardrail, and
a log that learns from outcomes alone would learn that first.

## The attempt trail

A finding key is stable by construction: the ratchet depends on it, so
`src/checks/complexity.rs` keys a violation by `path:symbol` rather than by
line. A key therefore survives the edits that fail to resolve it.

That gives a failed attempt for free, with no model input and no heuristics. The
same key present in two consecutive `sf check` runs, with a non-empty working
tree diff between them, is one attempt that did not move it. Record the diff
summary against the key, cap the list, and the report stops repeating itself:

| attempt | what the finding renders |
| --- | --- |
| 1 | the rule's `fix`, as today |
| 2 and 3 | plus the edits already tried against this key |
| 4 and up | plus the closest worked escape |

The trail is about an in-progress attempt, so it belongs to the working tree.
It is written under a gitignored path and stays out of the scope of
`L2.FACTORY_CONFIG_IS_LOCKED`, or every failed attempt becomes an edit to a
locked file.

## The escape is the minimal hunk

Recording the diff that preceded a green records coincidence. Most of a real
diff is co-changed noise, and "`src/orders/service.rs` changed" teaches nothing.

The causal subset is computable here, because `sf check --rule` already exists
and a single rule is cheap. On the transition, bisect the changed hunks against
that one key: the hunks whose removal brings the finding back are the fix, and
everything else is not. What gets stored is that minimal before and after, which
is the artifact a weak model can actually copy.

The cost is bounded by construction. Minimisation runs once, locally, on one
rule, with a ceiling on passes and a fallback to the diff of the file that
carried the finding's location.

## Escapes are classified, never trusted

A minimal hunk that lands in the scope of `L2.FACTORY_CONFIG_IS_LOCKED`, in the
ratchet, or in the policy is a weakening, not a fix. Those are the escapes with
the highest success rate, which is exactly why they can never be rendered as
advice. They are recorded as something for review.

## The fixture grows a green side

An escape log is empty until somebody has been stuck. This repository already
ships the other half of the pair: `src/fixtures.rs` holds the smallest
repository that violates each rule, in every language the rule claims, and
`sf verify` proves the rule fires on it.

A `fixed:` variant on `Fixture`, with `sf verify` asserting that the repair
clears the rule, gives every rule a proven before and after inside the binary
from the first run. Cold start solved by proof rather than by accumulation, and
the method extends cleanly: every check has a mutation that proves it fires and
a repair that proves it clears.

## Retrieval

Search is exact on the rule id and fuzzy only inside that bucket, ranked by
proximity, because the key is structured: same key, then same file, then same
directory, then any escape for the rule, then the rule's `fixed:` fixture. An
entry that is surfaced repeatedly and never precedes a green on its rule is
evicted, and each rule keeps a small ceiling of entries.

Decay is not only eviction. An escape that keeps helping is evidence that the
rule's `fix` prose is wrong, and its healthy end is to be promoted into that
prose and deleted. The log is a queue for improving the catalog, not a second
permanent channel competing with it.

## Deliberately not in scope

Sharing escapes between repositories. A literal hunk is directly applicable in
the repository it came from and means little outside it, and a bad
generalisation is worth less than a concrete example. Templating the hunk waits
until the local case is measured.

**Exit condition:** a model that has failed three times on the same finding key
sees, in the `sf check` output, both the edits it already tried and a minimal
worked diff that cleared the same rule, and no escape whose minimal hunks touch
the policy, the ratchet or a lock is ever offered as one of those diffs.

## Acceptance criteria

- [ ] A failed attempt is derived from the tool's own observations: the same
      finding key in two consecutive runs with a non-empty tree diff between
      them, with no model input anywhere in the write path
      (proof: test:src/escapes.rs)
- [ ] The attempt trail lives under a gitignored path and is absent from the
      scope of `L2.FACTORY_CONFIG_IS_LOCKED`
      (proof: test:src/escapes.rs)
- [ ] The rendered finding changes once attempts accumulate against the same
      key, instead of repeating byte for byte
      (proof: test:src/report.rs)
- [ ] An escape is minimised to the hunks that flip the key: removing them from
      the green tree reproduces the finding, removing any other hunk does not
      (proof: test:src/escapes.rs)
- [ ] Minimisation is bounded by a pass ceiling and falls back to the diff of
      the file carrying the finding's location
      (proof: test:src/escapes.rs)
- [ ] An escape whose minimal hunks touch the policy, the ratchet or a lock is
      stored as a weakening and never rendered as advice
      (proof: test:src/escapes.rs)
- [ ] Every fixture carries a `fixed:` variant, and `sf verify` fails both when
      the mutation does not trip the rule and when the repair does not clear it
      (proof: test:src/verify.rs)
- [ ] Retrieval ranks by proximity and never returns an escape for a different
      rule
      (proof: test:src/escapes.rs)
- [ ] An entry surfaced repeatedly without ever preceding a green on its rule is
      evicted, and each rule's store has a ceiling
      (proof: test:src/escapes.rs)
- [ ] A model looping on a real finding in this repository gets out with the log
      where it did not without it
      (proof: assertion:escape.breaks_a_real_loop)
- [ ] No model writes to the escape log
      (proof: unspecified:an absence, enforced by review of the diff that adds
      the writer)
- [ ] A recurring escape is promoted into the rule's `fix` prose and deleted
      (proof: deferred:needs the log to have run long enough to have produced a
      recurring entry)
