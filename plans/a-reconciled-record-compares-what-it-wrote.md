---
sources:
  - { id: S1, kind: ticket, ref: "#57", said: "creation writes the record's declared shape; the later unchanged decision reads fewer fields, so the rest drift forever. The update often writes the same incomplete subset, so it is not the reference" }
  - { id: S2, kind: code, ref: "src/catalog.rs", said: "`CheckKind`: shape matches one target, forwarder joins imported and forwarded names, nested compares containment; dispatch (`run_one` in src/checks/mod.rs) has no write/read field-set check" }
  - { id: S3, kind: decision, said: "add the smallest relational engine comparing an insert with a reconciliation decision in the same scope, rather than widening a pattern until it resembles one" }
  - { id: S4, kind: decision, said: "per-language specs identify record writes, the identity matching an external record, and the unchanged branch; the engine subtracts identity fields from creation fields minus decision reads and reports the remainder at both locations" }
  - { id: S5, kind: decision, said: "creation is authoritative; updates are evidence of mutation, never the expected set. Multiple creation sites compare separately, and shape disagreement is reported before any reconciliation result" }
  - { id: S6, kind: decision, said: "spreads, computed keys and helper builders whose fields cannot be recovered make the instance unsupported, not clean" }
  - { id: S7, kind: decision, said: "a policy instance names the create callee, lookup/reconcile scope and identity fields for one record family" }
  - { id: S8, kind: decision, said: "the first TypeScript adapter handles object-literal inserts and direct/member reads inside one function or module-local reconciliation path" }
  - { id: S9, kind: decision, said: "the mutation reproduces the payment/type failure and carries its repair; the measured repository must give the one known finding and no false positives before the instance is enabled" }
touches:
  - src/checks/record_consistency.rs
  - src/checks/mod.rs
  - src/catalog.rs
  - catalog/**
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "an object-literal creation writing identity plus three data fields, and an unchanged decision reading only two, reports the omitted field at both source locations", proof: "test:src/checks/record_consistency.rs", sources: [S1, S2, S3, S4, S7, S8] }
  - { id: A2, statement: "identity fields are subtracted explicitly, and update fields are never used as the expected record shape", proof: "test:src/checks/record_consistency.rs", sources: [S4, S5] }
  - { id: A3, statement: "adding the omitted field to the comparison clears the mutation fixture without changing the insert", proof: "test:src/fixtures.rs", sources: [S9] }
  - { id: A4, statement: "divergent creation shapes, computed keys, spreads and unresolved builders are named unsupported or ambiguous, not rendered as a clean comparison", proof: "test:src/checks/record_consistency.rs", sources: [S5, S6] }
  - { id: A5, statement: "the measured source repository yields the known payment-type finding, zero false positives, and clears when that field joins the decision", proof: "unspecified:the source repository and adjudication live outside this repository", sources: [S9] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A3)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A3)" }
  - { mechanism: inertness, answer: n/a, why: "an instance whose fields cannot be recovered reports unsupported (A4), not clean and not inert" }
  - { mechanism: overlay, answer: n/a, why: "a kind that reads the code runs unchanged under an overlay, as shape, forwarder and nested do in src/checks/mod.rs" }
failures:
  - { step: "field extraction", when: "a spread, computed key or unrecoverable builder", goes_to: "instance named unsupported, not clean" }
  - { step: "creation comparison", when: "creation sites disagree on shape", goes_to: "disagreement reported before any reconciliation result" }
cut:
  - "cross-file provenance: waits for a symbol graph, which this work does not imply exists"
  - "enabling the instance before the measured repository gives one known finding and no false positives"
exit: "a configured TypeScript reconciliation path cannot call a record unchanged while ignoring a statically known non-identity field its own creation wrote, and uncertainty in the field set is visible rather than reported as coverage"
---
# A reconciled record compares what it wrote

Issue #57's defect lives only in the difference between the fields a creation writes and the fields an "unchanged" decision reads, and no current check kind can state that relation.
