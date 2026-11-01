---
sources:
  - { id: S1, kind: ticket, ref: "#54", said: "one exported capability is called from several paths; most calls are dominated by a policy predicate, another reaches it unguarded" }
  - { id: S2, kind: decision, said: "not helper adoption: callers may share no input type and no helper; what must agree is the control-flow condition around one callee" }
  - { id: S3, kind: reference, ref: "plans/siblings-share-the-helper.md", said: "owns the other half of issue #54" }
  - { id: S4, kind: code, ref: "src/catalog.rs", said: "catalog relations (`LangQuery`, `ForwarderQuery`, `NestedQuery`, `CheckKind`) are placement, containment and local name joins; none establishes that a condition dominates a call" }
  - { id: S5, kind: reference, ref: "plans/external-data-is-checked-not-cast.md", said: "introduces the local control-flow/dominance vocabulary this plan reuses rather than a second, subtly different walk" }
  - { id: S6, kind: decision, said: "a guard-agreement kind: a language spec captures the target call and candidate guards; per function the engine records the normalised conditions dominating the call" }
  - { id: S7, kind: decision, said: "across call sites of one resolved local/exported target, report a site missing a guard the measured majority shares, showing guarded and unguarded callers together; an isolated call has no consensus" }
  - { id: S8, kind: decision, said: "boolean forms normalise only where the adapter proves equivalence; negation, early return and enclosing conditions are covered; helper predicates compare by resolved local symbol and arguments" }
  - { id: S9, kind: decision, said: "dynamic dispatch, unresolvable cross-file aliases and side-effecting conditions are marked unsupported, not flattened into a string comparison" }
  - { id: S10, kind: decision, said: "the fixture has three callers, then adds a fourth unguarded path after the first outlier is repaired; the check reports until every caller agrees, so a point fix is not the next divergence" }
  - { id: S11, kind: ticket, ref: "#54", said: "corpus enablement requires the two measured unguarded paths and zero false positives" }
touches:
  - src/checks/guard_agreement.rs
  - src/checks/mod.rs
  - src/catalog.rs
  - catalog/**
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "calls to one locally resolvable target are compared by the predicates dominating them, including early-return and enclosing-condition forms", proof: "test:src/checks/guard_agreement.rs", sources: [S1, S2, S3, S4, S5, S6, S8] }
  - { id: A2, statement: "a caller missing a guard the measured majority shares is reported with both the guarded and unguarded call sites named", proof: "test:src/checks/guard_agreement.rs", sources: [S7] }
  - { id: A3, statement: "an isolated call, an evenly split group and an unresolved dynamic target are not presented as agreement violations", proof: "test:src/checks/guard_agreement.rs", sources: [S7, S9] }
  - { id: A4, statement: "repairing one outlier while a second unguarded caller remains keeps the fixture red until every caller agrees", proof: "test:src/fixtures.rs", sources: [S10] }
  - { id: A5, statement: "the measured source repository yields both known unguarded capability paths and no false positives before default enablement", proof: "unspecified:the source repository and adjudication live outside this repository", sources: [S11] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A4)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A4)" }
  - { mechanism: inertness, answer: n/a, why: "unresolvable control flow is counted unsupported (A3), not reported clean or inert" }
  - { mechanism: overlay, answer: n/a, why: "a kind that reads the code runs unchanged under an overlay, as shape, forwarder and nested do in src/checks/mod.rs" }
failures:
  - { step: "guard comparison", when: "dynamic dispatch, an unresolvable alias or a side-effecting condition", goes_to: "marked unsupported, not string-compared" }
cut:
  - "a second control-flow walk: refused, reuse the dominance vocabulary (S5)"
  - "default enablement before the corpus gives both known paths and zero false positives (S11)"
exit: "callers of one resolvable capability cannot silently disagree about the guard that enables it, a point fix does not hide the next unguarded path, and unsupported control flow is counted rather than called safe"
---
# Calls to one capability share the guard

Issue #54's second shape: most callers of one capability pass a policy predicate and one reaches it unguarded. No catalog relation can say a condition dominates a call.
