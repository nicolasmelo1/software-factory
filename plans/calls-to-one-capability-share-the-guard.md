# Calls to one capability share the guard

Issue #54 also records a second shape: one exported capability is called from
several paths, most calls are dominated by a policy predicate, and another
reaches it unguarded. This is not helper adoption. The functions may share no
input type and call no common helper; what must agree is the control-flow
condition around one callee. [siblings-share-the-helper.md](siblings-share-the-helper.md)
owns the other half of the issue.

Tree-sitter queries can capture calls and predicates, but the existing catalog
relations are placement, containment and local name joins
(`src/catalog.rs:47-76`, `:88-121`). None establishes that a condition
dominates a call. The boundary-validation work in
[external-data-is-checked-not-cast.md](external-data-is-checked-not-cast.md)
introduces the local control-flow/dominance vocabulary this plan reuses rather
than implementing a second, subtly different walk.

## What changes

**A guard-agreement check kind.** A language spec captures the target call and
candidate guard predicates. Within each function, the engine records the
normalised conditions that dominate the call. Across call sites for the same
resolved local/exported target, it reports a site missing a guard shared by the
measured majority. The report shows guarded and unguarded callers together;
an isolated call has no consensus to violate.

**Agreement is structural, not textual.** Equivalent boolean forms normalise
only where the language adapter can prove equivalence. Negation, early return
and enclosing condition forms are covered; helper predicates compare by
resolved local symbol and arguments. Dynamic dispatch, cross-file aliases the
engine cannot resolve and side-effecting conditions are marked unsupported,
not flattened into a string comparison.

**Fixes trigger the whole sweep.** The fixture carries three callers, then adds
a fourth unguarded path after the first outlier is repaired. The check must keep
reporting until every caller in the group agrees, preventing a point fix from
becoming the next divergence. Corpus enablement requires the two measured
unguarded paths from issue #54 and zero false positives.

## Acceptance criteria

- [ ] Calls to one locally resolvable target are compared by the predicates
      that dominate them, including early-return and enclosing-condition forms
      (proof: test:src/checks/guard_agreement.rs)
- [ ] A caller missing a guard shared by the measured majority is reported with
      both the guarded and unguarded call sites named
      (proof: test:src/checks/guard_agreement.rs)
- [ ] An isolated call, an evenly split group and an unresolved dynamic target
      are not presented as agreement violations
      (proof: test:src/checks/guard_agreement.rs)
- [ ] Repairing one outlier while a second unguarded caller remains keeps the
      fixture red until every caller agrees
      (proof: test:src/fixtures.rs)
- [ ] The measured source repository yields both known unguarded capability
      paths and no false positives before default enablement
      (proof: unspecified:the source repository and adjudication live outside
      this repository)

**Exit condition:** callers of one resolvable capability cannot silently
disagree about the guard that enables it, a point fix does not hide the next
unguarded path, and unsupported control flow is counted rather than called
safe.
