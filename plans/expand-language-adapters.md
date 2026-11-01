---
sources:
  - { id: S1, kind: code, ref: "src/lang.rs", said: "sf parses Python, TypeScript/TSX, Go, Rust, Ruby; a language is a tree-sitter grammar here (function-opening and branching node kinds, boolean operators) plus one query per L0 rule with something to say" }
  - { id: S2, kind: code, ref: "0a285f9", said: "Ruby is done: queries added, two rules whose Ruby form was judged dishonest left without one, and a real grammar bug exposed in the complexity engine" }
  - { id: S3, kind: decision, said: "Java, C# and PHP are the next candidates; Kotlin and Swift matter for mobile products" }
  - { id: S4, kind: decision, said: "a rule with no query for a language does not apply to it, correctly; L0.PERSISTENCE_STAYS_IN_REPOSITORIES means nothing where the data layer is no session object" }
  - { id: S5, kind: decision, said: "a clean run is not proof: a rule matching nothing also produces one, so a maintainer of the language must agree a finding is real" }
touches:
  - src/lang.rs
  - catalog/L0/*.yaml
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "the adapter declares the grammar vocabulary to read functions, branches and boolean operators without inventing a shape the language lacks", proof: "test:src/lang.rs", sources: [S1, S3, S4] }
  - { id: A2, statement: "each rule with honest vocabulary in the language carries a query, and its mutation fixture proves the query fires", proof: "test:src/fixtures.rs", sources: [S2, S4] }
  - { id: A3, statement: "an initialized repository in the language runs `sf verify` and produces a finding its maintainer agrees is real", proof: "assertion:adapter.maintainer_accepted_finding", sources: [S5] }
crosses:
  - { mechanism: languages, answer: works, proof: "test:src/lang.rs (A1)" }
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A2)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A2)" }
cut:
  - "inventing a query so the coverage table looks full: refused, it makes findings nobody believes (S4)"
exit: "a repository in the new language runs `sf init`, `sf verify` reports every enabled rule firing on its fixture, and `sf check` produces at least one finding a maintainer of that language agrees is real, not merely a clean run"
---
# Expand the language adapters

Java, C# and PHP are next, Kotlin and Swift for mobile products. A new language is a grammar plus queries only where a rule has honest vocabulary; a clean run there proves nothing.
