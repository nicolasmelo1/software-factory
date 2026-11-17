---
sources:
  - { id: S1, kind: decision, said: "L6 needs a rule that catches duplicated code inside a repository; the detector is jscpd in every language the rule covers. Native detectors (PMD CPD, golangci-lint dupl, pylint symilar, flay) were weighed and are not listed" }
  - { id: S2, kind: code, ref: "catalog/L6/dead-code-is-detected.yaml", said: "the nearest rule: kind toolchain, ratchet allowlist, a language-neutral concern with one tool list per language. No duplication rule exists in catalog/ today" }
  - { id: S3, kind: code, ref: "src/checks/toolchain.rs", said: "the check is a substring search over CI and task runner files with comment lines dropped; it cannot tell which language a tool was pointed at, nor whether it can exit non-zero" }
  - { id: S4, kind: reference, ref: "https://github.com/kucherenko/jscpd/tree/master-v4/apps/jscpd", said: "jscpd v4 exits 0 when it finds clones unless --threshold or --exitCode is set; defaults are 50 tokens and 5 lines; --format limits the languages read" }
  - { id: S5, kind: reference, ref: "https://github.com/kucherenko/jscpd", said: "jscpd reads 224 formats including typescript, python, go, rust and ruby; v5 is a Rust binary and adds --fail-on-new-clones against a baseline" }
  - { id: S6, kind: reference, ref: "plans/a-hazard-tool-is-proven-to-fail.md", said: "a hazard tool that cannot exit non-zero reads as protection; the falsifier proves the tool goes red on a planted defect" }
  - { id: S7, kind: decision, said: "a repository with history already holds clones; the build blocks the next one through a duplication ceiling that may only fall, not through a clean slate" }
touches:
  - catalog/L6/duplicate-code-is-detected.yaml
  - src/fixtures.rs
  - src/checks/toolchain.rs
  - docs/rules.md
  - docs/reference/**
  - .software-factory/**
  - .github/workflows/software-factory.yml
  - .jscpd.json
  - plans/next-steps.md
criteria:
  - { id: A1, statement: "a duplicate-code hazard rule ships as a toolchain rule whose tools list jscpd, and only jscpd, for typescript, python, go, rust and ruby", proof: "test:src/fixtures.rs", sources: [S1, S2, S5] }
  - { id: A2, statement: "the fixture trips the rule in every language it declares, and a workflow that runs jscpd clears it", proof: "test:src/fixtures.rs", sources: [S2] }
  - { id: A3, statement: "the scope matches the dead-code rule's runner files, so a jscpd call in a workflow, task runner or package script satisfies it and a .jscpd.json alone does not", proof: "test:src/checks/toolchain.rs", sources: [S3] }
  - { id: A4, statement: "the fix says jscpd exits 0 on clones unless --threshold or --exitCode is set, and names both", proof: "unspecified:prose; held by the falsifier once plan 14 lands", sources: [S4, S6] }
  - { id: A5, statement: "this repository enables the rule over its Rust sources with a measured jscpd ceiling, excluding target and .software-factory/mutations, and its own CI goes red when the ceiling is crossed", proof: "unspecified:measured on this repository's CI before enablement", sources: [S5, S7] }
  - { id: A6, statement: "docs/rules.md carries the rule's section, and `sf verify` and `sf check` are green after fixtures, docs, ratchet and lock", proof: "test:src/fixtures.rs", sources: [S2] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A1, A2)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A2)" }
  - { mechanism: inertness, answer: works, proof: "test:src/fixtures.rs (A1); every declared language has a tools entry, so L5.NO_INERT_RULE has nothing to report" }
  - { mechanism: generated-docs, answer: works, proof: "`sf docs --check` after the rule's section lands in docs/rules.md (A6)" }
  - { mechanism: toolchain, answer: works, proof: "test:src/checks/toolchain.rs (A3)" }
  - { mechanism: tightening, answer: works, proof: "deferred:the jscpd ceiling in A5 is a number that may only fall, held by L2.POLICY_ONLY_TIGHTENS once it lives in policy" }
guarantee:
  - { holds: "a repository with this rule enabled runs jscpd somewhere it actually executes", not_against: "jscpd run without --threshold or --exitCode, which exits 0, until plan 14's falsifier proves it fails" }
  - { holds: "copied blocks above jscpd's token and line floor are found", not_against: "logic rewritten with a different shape, which siblings-share-the-helper owns" }
failures:
  - { step: "toolchain match", when: "one jscpd call covers only some of the declared languages through --format", goes_to: "passes every language anyway; recorded as a known gap of the substring check, not fixed here" }
  - { step: "enablement here", when: "jscpd reports the repetitive Fixture table in src/fixtures.rs", goes_to: "an ignore with a written reason in .jscpd.json, never a raised ceiling without one" }
cut:
  - "a clone detector inside sf: refused, L6 owns no scanner and names the tool (docs/method.md, L6)"
  - "native detectors per language: jscpd reads all five, and each extra accepted name widens the substring match for no added coverage (S1)"
  - "semantic duplication and helper adoption: owned by plans/siblings-share-the-helper.md"
  - "duplication across repositories: not a single-repository hazard"
  - "proving jscpd exits non-zero on a planted clone: owned by a-hazard-tool-is-proven-to-fail (S6)"
exit: "a repository declaring any of typescript, python, go, rust or ruby is told by `sf check` when nothing runs jscpd, and this repository's own CI fails when its Rust sources cross the measured duplication ceiling"
---
# Duplicate code is detected

Agents copy before they extract: the second caller gets a pasted block, not a helper. No L6 rule looks for it. This adds one requiring jscpd to run, for every language it covers, without moving detection into `sf`.
