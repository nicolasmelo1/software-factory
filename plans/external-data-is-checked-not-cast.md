---
sources:
  - { id: S1, kind: ticket, ref: "#56", said: "an external value is cast to a concrete shape and fields are read through it without runtime validation: typed only because the author told the compiler to stop asking" }
  - { id: S2, kind: code, ref: "src/catalog.rs", said: "`shape` (`LangQuery`, `CheckKind::Shape`) reports only a node and its placement; it cannot follow the value, enumerate fields read through it, or decide whether a guard dominates each read" }
  - { id: S3, kind: ticket, ref: "#56", said: "a text-pattern ban reports every cast, including boundaries that do validate: the false-positive failure the issue measured" }
  - { id: S4, kind: code, ref: "src/checks/forwarder.rs", said: "the nearest multi-query engine (`run`, `in_file`) joins two local shapes by name in one file: the extension point is a typed check kind with a comparison queries cannot express" }
  - { id: S5, kind: decision, said: "add the first local flow-sensitive engine rather than pretending one tree-sitter match is a data flow analysis" }
  - { id: S6, kind: decision, said: "specs capture external source, narrowing cast/assertion, field reads through the narrowed value, guards/validators; joined in one function over its control flow, each read no validation dominates is reported" }
  - { id: S7, kind: decision, said: "no whole-program claims: aliases escaping the function and dynamic field names are reported as unsupported analysis, not silently certified" }
  - { id: S8, kind: decision, said: "built-in guards cover native type/null/presence checks; policy may name validator functions whose successful return establishes the whole value or named fields" }
  - { id: S9, kind: decision, said: "truthiness guard, optional-chain fallback, schema parser and helper call are tested separately; a helper is not accepted because its name sounds like validation" }
  - { id: S10, kind: code, ref: "src/lang.rs", said: "`Lang::name` carries TypeScript and TSX through one rule surface; the issue and corpus are TypeScript-shaped, so TypeScript is the first adapter" }
  - { id: S11, kind: decision, said: "other languages stay absent until their boundary and narrowing vocabulary is measured; a language with no spec is not reported as covered" }
  - { id: S12, kind: decision, said: "the mutation has one unsafe cast and adjacent safe forms; default enablement waits until, over #56's repository, it finds the known defect, stays quiet on two validated casts, and the measurement is in the rule prose" }
touches:
  - src/checks/boundary_validation.rs
  - src/checks/mod.rs
  - src/catalog.rs
  - catalog/**
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "a field read through a concrete cast of request, message, parsed document or `unknown` input is reported when no dominating validation establishes that field", proof: "test:src/checks/boundary_validation.rs", sources: [S1, S2, S4, S5, S6] }
  - { id: A2, statement: "native type/presence guards, configured validator calls and schema parse results clear only the reads they actually establish", proof: "test:src/checks/boundary_validation.rs", sources: [S8, S9] }
  - { id: A3, statement: "a guard on one branch does not certify a read reachable from another, and a guard after the read does not certify the earlier use", proof: "test:src/checks/boundary_validation.rs", sources: [S6] }
  - { id: A4, statement: "aliases escaping local analysis and dynamic field reads are rendered unsupported rather than counted as safe", proof: "test:src/checks/boundary_validation.rs", sources: [S7] }
  - { id: A5, statement: "the TypeScript fixture trips on the unsafe cast and stays quiet on the guarded and validator-backed forms beside it", proof: "test:src/fixtures.rs", sources: [S3, S10, S11, S12] }
  - { id: A6, statement: "the measured source repository yields the known true finding and none on either already-validated cast before default enablement", proof: "unspecified:the source repository and adjudication live outside this repository", sources: [S3, S12] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A5)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A5)" }
  - { mechanism: inertness, answer: works, proof: "deferred:no criterion yet; S11 says a language with no spec is not reported as covered" }
  - { mechanism: overlay, answer: n/a, why: "a kind that reads the code runs unchanged under an overlay, as shape, forwarder and nested do in src/checks/mod.rs" }
failures:
  - { step: "flow analysis", when: "an alias escapes the function or a field name is dynamic", goes_to: "reported as unsupported analysis, never certified" }
cut:
  - "a text-pattern ban on casts: refused, it reports validating boundaries too (S3)"
  - "whole-program analysis: not claimed (S7)"
  - "languages other than TypeScript until their vocabulary is measured (S11)"
exit: "`sf check` identifies a field trusted through an external TypeScript cast only when no dominating check or declared validator establishes it, catches the measured defect, and does not turn every cast into advice a maintainer learns to ignore"
---
# External data is checked, not cast

Issue #56: a cast tells the compiler to stop asking, and fields are trusted with no runtime validation. Matching the cast reports every boundary, validated or not, so the check has to follow the value.
