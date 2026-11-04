---
base: 2d6f9de2b3eb1400b9133e12a0c1d7ebd4e6d3c9
sources:
  - { id: S1, kind: decision, said: "a TypeScript-only hazard rule: something proves the typed code meets its types, with TScert as the tool; not a rule for every language" }
  - { id: S2, kind: reference, ref: "https://github.com/cristianoc/TScert", said: "TScert accepts a TypeScript function only with a certificate a Lean-proved checker validates; it refuses any and decides nothing from as or !" }
  - { id: S7, kind: reference, ref: "https://github.com/cristianoc/TScert/blob/main/pipeline/verify-source.mts", said: "make check FILE=… runs verify-source.mts, which exits 0 only when the file verifies; make census records and exits 0 whatever it found" }
  - { id: S3, kind: reference, ref: "https://github.com/cristianoc/TScert/blob/main/LICENSE", said: "NON-AI MIT: the code may not be used to train or improve machine-learning systems; it is invoked, never vendored" }
  - { id: S4, kind: code, ref: "src/checks/toolchain.rs", said: "a toolchain rule names a concern and per-language tools, and finds one of them in what the repository runs; a language with no entry is not a finding" }
  - { id: S5, kind: code, ref: "catalog/L6/dead-code-is-detected.yaml", said: "the shape of a toolchain hazard rule: scope over CI and task runners, tools per language" }
  - { id: S6, kind: decision, said: "the rule is opt-in: TScert needs Lean, OCaml and Dune and covers a subset of TypeScript, so it is off unless a repository enables it; this repository, which is Rust, switches it off in writing" }
touches:
  - catalog/L6/type-soundness-is-certified.yaml
  - src/catalog.rs
  - src/fixtures.rs
  - .software-factory/mutations/**
  - .software-factory/policy.yaml
  - .software-factory/catalog.lock.json
  - .software-factory/locks/**
  - .software-factory/ratchet.yaml
  - docs/rules.md
  - README.md
criteria:
  - { id: A1, statement: "a toolchain rule in catalog/L6/type-soundness-is-certified.yaml whose only tools are TypeScript's, and whose fix names the TScert command that exits non-zero on an unverified file and says it is invoked, never vendored", proof: "test:src/catalog.rs", sources: [S1, S2, S3, S4, S5, S7] }
  - { id: A2, statement: "its mutation fixture, a CI that runs no certifier, trips it for typescript and for no other language", proof: "test:src/fixtures.rs", sources: [S4] }
  - { id: A3, statement: "this repository switches it off with the reason written in docs/rules.md, and sf verify stays green", proof: "unspecified:sf verify and sf check over this repository", sources: [S6] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/catalog.rs; registered in BUILTIN" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs" }
  - { mechanism: generated-docs, answer: works, proof: "unspecified:sf docs regenerates the README table and the rules.md section" }
  - { mechanism: toolchain, answer: n/a, why: "the checker is unchanged; the rule only declares tools for it" }
  - { mechanism: inertness, answer: works, proof: "unspecified:switched off here in writing, so L5.NO_INERT_RULE stays green" }
guarantee:
  - { holds: "a TypeScript repository that enables the rule is told when nothing it runs certifies its types", not_against: "a CI that names the TScert command on files that prove nothing useful, or that runs it and ignores the exit code: the rule reads names, not outcomes" }
  - { holds: "the command the fix names exits non-zero on a file that does not verify", not_against: "a TScert release that changes that exit code" }
failures:
  - { step: "sf check on a TypeScript repository", when: "the rule is enabled and no CI file names the certifier", goes_to: "one finding for typescript, naming the expected commands" }
cut:
  - "a rule for languages other than TypeScript: TScert is TypeScript's (S1)"
  - "vendoring or installing TScert: invoked by the repository's own CI only (S3)"
  - "enabling it by default or through the interview: opt-in (S6)"
exit: "a TypeScript repository that enables the rule is red until its CI runs TScert's check, and this repository says in writing why it is off here"
---
# Typed code is certified against its types

TypeScript's checker is unsound by design, and an agent reaches for `as` and `!`
exactly where the shape was unclear. TScert accepts a function only with a
proof a Lean-verified checker validates; this makes running it a hazard a
TypeScript repository can opt into.
