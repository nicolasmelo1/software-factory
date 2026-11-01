---
sources:
  - { id: S1, kind: ticket, ref: "#54", said: "sibling functions consume the same shape; most use the local helper encoding one decision, and one open-codes or omits it" }
  - { id: S2, kind: reference, ref: "plans/calls-to-one-capability-share-the-guard.md", said: "owns guard agreement between callers; the two share provenance, not an engine or an exit condition" }
  - { id: S3, kind: friction, said: "a name-family heuristic was measured and found nothing; a group must not be inferred from similar spelling" }
  - { id: S4, kind: code, ref: "src/checks/forwarder.rs", said: "forwarder (`in_file`, `alias_map`) maps imported and local names, then compares function and callee: captures are joined semantically. Adoption needs another relation: functions grouped by what they consume, then a helper the group uses" }
  - { id: S5, kind: decision, said: "queries capture functions, their consumed parameter type or common input field, and local helper calls; group by consumed shape, find a helper a configurable minimum adopts, report a sibling doing the role without it" }
  - { id: S6, kind: decision, said: "the finding names the group, the adopting siblings and the outlier, so a maintainer can reject a bad grouping rather than trust a score" }
  - { id: S7, kind: decision, said: "a missing call is reported only when the outlier reads the input fields the helper resolves or normalises; a same-type function never touching them is not a sibling" }
  - { id: S8, kind: decision, said: "wrappers and delegated calls are followed only within the file; uncertainty stays silent and is counted in the corpus report" }
  - { id: S9, kind: decision, said: "the fixture proves the fallback-id defect and a legitimate sibling that does not need the helper" }
  - { id: S10, kind: ticket, ref: "#54", said: "before default enablement the grouping and threshold run over #54's repository: the known outlier appears, no false positives, groups considered and skipped as uncertain recorded" }
touches:
  - src/checks/sibling_adoption.rs
  - src/checks/mod.rs
  - src/catalog.rs
  - catalog/**
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "functions are grouped by a shared consumed type or input field, never by a name prefix alone", proof: "test:src/checks/sibling_adoption.rs", sources: [S1, S2, S3, S4, S5] }
  - { id: A2, statement: "a sibling reading the helper's decision inputs without calling the helper is reported with the adopting siblings and helper named", proof: "test:src/checks/sibling_adoption.rs", sources: [S6, S7] }
  - { id: A3, statement: "a same-type function not consuming those inputs, and a group below the measured threshold, stay quiet", proof: "test:src/checks/sibling_adoption.rs", sources: [S7, S8] }
  - { id: A4, statement: "the mutation reproduces a delete handler reading only the primary id, while its repair uses the fallback helper", proof: "test:src/fixtures.rs", sources: [S9] }
  - { id: A5, statement: "the measured corpus finds the known outlier, records every skipped uncertain group, and produces no false positives before enablement", proof: "unspecified:the source repository and adjudication live outside this repository", sources: [S8, S10] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A4)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A4)" }
  - { mechanism: inertness, answer: n/a, why: "uncertain groups stay silent and are counted in the corpus report (A5); no inert path changes" }
  - { mechanism: overlay, answer: n/a, why: "a kind that reads the code runs unchanged under an overlay, as shape, forwarder and nested do in src/checks/mod.rs" }
failures:
  - { step: "grouping", when: "a wrapper or delegated call cannot be followed within the file", goes_to: "silent, counted in the corpus report" }
cut:
  - "inferring a group from name spelling: refused, measured to find nothing (S3)"
  - "guard agreement between callers: owned by the sibling plan (S2)"
exit: "a measured sibling group cannot leave one handler open-coding a decision the others take from a shared helper without `sf check` naming helper, group and outlier, and the rule has fired on real code, not only its fixture"
---
# Siblings share the helper

Issue #54's first defect: siblings consuming one shape mostly call the helper that encodes a decision, and one open-codes or omits it. Name-family heuristics found nothing, so grouping must come from what functions consume.
