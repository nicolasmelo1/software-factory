---
sources:
  - { id: S1, kind: decision, said: "L6 owns no scanner: a rule names a language-neutral concern, the check asserts something covering it runs, the tool is the per-language adapter. This plan keeps that split" }
  - { id: S2, kind: friction, said: "the first adopting product runs `vitest bench` with a committed baseline in CI, as the fix prescribes; against a baseline claiming 20x throughput it prints ⇓ [0.05x] and exits 0" }
  - { id: S3, kind: code, ref: "catalog/L6/performance-regression-is-guarded.yaml", said: "enabled here and in that product; its why: a committed baseline turns a slowdown into a failed build; its fix names a command that reports rather than judges" }
  - { id: S4, kind: code, ref: "src/checks/toolchain.rs", said: "toolchain finds the tool's name in workflows and manifests; L5.NO_INERT_RULE reads rule config. Neither can know the named process exits zero on its defect" }
  - { id: S5, kind: decision, said: "a hazard tool's falsifier: smallest repository with the defect planted, the command, exit non-zero with the defect and zero without; the object sf verify builds for catalog rules, pointed at the delegate" }
  - { id: S6, kind: decision, ref: "src/checks/command.rs", said: "sf verify runs falsifiers, not sf check: tools are slow and reach the network; command machinery and its --allow-commands gate already exist" }
  - { id: S7, kind: decision, said: "a hazard whose defect cannot be planted where the run happens (needs hardware, a paid feed) declares that with a reason, as the rest of the catalog does" }
  - { id: S8, kind: decision, said: "the perf fix stops naming a report-only command; for TypeScript it reads the comparison and exits non-zero past a declared threshold, a ceiling under L2.POLICY_ONLY_TIGHTENS" }
  - { id: S9, kind: friction, said: "the product's baseline was measured under a path no longer on any disk and is compared against whichever CI runner ran that morning; speed expires when the machine moves" }
  - { id: S10, kind: reference, ref: "Bend's benchmark tree", said: "one pinned baseline file per hardware, medians of three runs: the missing environment field, not a new idea" }
touches:
  - src/checks/hazard.rs
  - src/checks/command.rs
  - src/checks/toolchain.rs
  - src/verify.rs
  - src/fixtures.rs
  - src/catalog.rs
  - catalog/L6/*.yaml
criteria:
  - { id: A1, statement: "a hazard rule declares a falsifier, and `sf verify` fails when the declared command exits zero against the planted defect", proof: "test:src/checks/hazard.rs", sources: [S1, S2, S4, S5] }
  - { id: A2, statement: "the same falsifier must pass without the defect, so a command that always fails is refused too", proof: "test:src/checks/hazard.rs", sources: [S5] }
  - { id: A3, statement: "a hazard rule with no falsifier and no written reason is a finding; one with a reason is not", proof: "test:src/checks/hazard.rs", sources: [S7] }
  - { id: A4, statement: "`sf verify` without `--allow-commands` reports the falsifier as unrun rather than passed", proof: "test:src/checks/command.rs", sources: [S6] }
  - { id: A5, statement: "`L6.PERFORMANCE_REGRESSION_IS_GUARDED` names a command that exits non-zero on a planted slowdown, and its own falsifier proves it", proof: "test:src/fixtures.rs", sources: [S2, S3, S8] }
  - { id: A6, statement: "a benchmark baseline carries the environment it was measured on; a comparison across two environments reports unavailable rather than a verdict", proof: "unspecified:where the field lives depends on whether the baseline stays a tool's own format or becomes an evidence artifact, and that is the first thing the work has to decide", sources: [S9, S10] }
crosses:
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A5)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/checks/hazard.rs (A1, A2)" }
  - { mechanism: commands, answer: works, proof: "test:src/checks/command.rs (A4)" }
  - { mechanism: toolchain, answer: works, proof: "test:src/checks/hazard.rs (A1); toolchain still only finds the name, the falsifier proves it fails" }
  - { mechanism: tightening, answer: works, proof: "deferred:A5's threshold is a ceiling held by L2.POLICY_ONLY_TIGHTENS" }
guarantee:
  - { holds: "a declared hazard tool goes non-zero on a planted defect and zero without it", not_against: "a tool that catches the planted defect and misses real ones, or a badly chosen threshold" }
failures:
  - { step: "falsifier run", when: "`--allow-commands` is absent", goes_to: "reported as unrun, never as passed" }
  - { step: "benchmark comparison", when: "baseline and run come from different environments", goes_to: "reported unavailable, not as a result" }
cut:
  - "moving scanners into L6 or changing the concern/adapter split: untouched (S1)"
  - "running falsifiers in sf check: refused, slow and networked (S6)"
  - "claiming the tool is good: a secret scanner catching the planted key may miss every real one"
exit: "a repository whose performance guard cannot fail is told so by `sf verify`, with the command it ran and the exit code it got, and the same run proves every other declared hazard tool goes red on a planted defect"
---
# A hazard tool is proven to fail

L6 checks that a tool covering a concern is named and run, never that it can return non-zero. The performance guard in the first adopting product exits 0 on a 20x slowdown, and every report reads that as protection.
