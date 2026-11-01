---
sources:
  - { id: S1, kind: code, ref: "c8a1db6", said: "`sf check --policy <dir>` shipped the read path, both refusals, and reports naming the overlay's path and digest with no ratchet; inapplicability did not ship, so it misreports" }
  - { id: S2, kind: friction, said: "a bare one-file Rust repository with no `.software-factory/`, governed from outside, got eighty-odd findings the code earned none of" }
  - { id: S3, kind: friction, said: "L4.EVERY_RULE_HAS_A_WHY resolved `docs.scan` against the target; L5.EVERY_CHECK_HAS_A_MUTATION_TEST missed fixtures under the overlay's `.software-factory/mutations/`" }
  - { id: S4, kind: reference, ref: "docs/design/a-gate-outlives-its-plan.md", said: "a gate's criteria document lives in the overlay's repository, so L3.GATE_COVERS_THE_PLAN reported gate `adoption`'s plan missing" }
  - { id: S5, kind: friction, said: "L2.DEPENDENCIES_CHANGE_DELIBERATELY reported a never-written lock nothing can clear, sf lock being refused under an overlay; ROOT_FILES_ARE_DECLARED, CATALOG_ONLY_TIGHTENS, NO_INERT_RULE failed alike" }
  - { id: S6, kind: code, ref: "src/checks/mod.rs", said: "`inapplicable_under_overlay` named three rules (GATE_HAS_FRESH_EVIDENCE, POLICY_ONLY_TIGHTENS, FACTORY_CONFIG_IS_LOCKED); every other factory-state rule read the target's absent state as a violation" }
  - { id: S7, kind: decision, said: "a hand-written list is a memory that goes stale; derive the set from the check kind and the paths it reads: evidence, fixtures, hash locks, catalog fingerprint, rule prose, the plan queue" }
  - { id: S8, kind: decision, said: "L4.ROOT_FILES_ARE_DECLARED is inapplicable under an overlay: its only fix writes into a repository the overlay may not write to; same for any rule whose remedy is a file sf init writes" }
  - { id: S9, kind: decision, said: "L5.NO_INERT_RULE composes rather than joins the set: it skips rules already reporting inapplicable instead of reading their silence as inertness" }
  - { id: S10, kind: reference, ref: "34cd141^:plans/policy-governs-a-repo-it-does-not-live-in.md", said: "the parent plan cited test:src/policy.rs, test:src/main.rs and test:src/verify.rs; none existed: the refusal was inline in cmd_check, main.rs had no tests, clap refused the writers untested" }
  - { id: S11, kind: friction, said: "ROOT_FILES_ARE_DECLARED, CATALOG_ONLY_TIGHTENS and NO_INERT_RULE failed over, respectively, the root allowlist, the catalog fingerprint, and a plans scope that matches nothing" }
touches:
  - src/checks/mod.rs
  - src/checks/cadence.rs
  - src/policy.rs
  - src/main.rs
  - src/verify.rs
criteria:
  - { id: A1, statement: "every rule whose state lives beside the policy reports inapplicable under an overlay with its reason, the set derived from check kind and paths read, not a list of ids", proof: "test:src/checks/mod.rs", sources: [S1, S3, S4, S5, S6, S7, S11], done: true }
  - { id: A2, statement: "an overlay run over a repository with no factory directory reports what the same policy vendored into it reports over the same code, differing only in the ratchet", proof: "test:src/checks/mod.rs", sources: [S2], done: true }
  - { id: A3, statement: "`L5.NO_INERT_RULE` skips a rule inapplicable under the overlay rather than reporting it inert", proof: "test:src/checks/mod.rs", sources: [S9], done: true }
  - { id: A4, statement: "`L4.ROOT_FILES_ARE_DECLARED` is inapplicable under an overlay, on the reasoning decided above", proof: "test:src/checks/mod.rs", sources: [S8], done: true }
  - { id: A5, statement: "`--policy` against a root carrying its own policy is refused by a test, not only by a message", proof: "test:src/policy.rs", sources: [S10], done: true }
  - { id: A6, statement: "`sf init`, `sf ratchet`, `sf lock`, `sf fixtures` and `sf seal` are proven to refuse the flag, read out of the clap definition rather than asserted in a comment", proof: "test:src/main.rs", sources: [S10], done: true }
  - { id: A7, statement: "`sf verify` proves the overlay's own rules fire in the repository the overlay lives in, so governing from outside cannot ship rules nothing trips", proof: "test:src/verify.rs", sources: [S10], done: true }
crosses:
  - { mechanism: overlay, answer: works, proof: "test:src/checks/mod.rs (A1, A2, A4)" }
  - { mechanism: inertness, answer: works, proof: "test:src/checks/mod.rs (A3)" }
  - { mechanism: plans, answer: works, proof: "test:src/checks/mod.rs (A1): plan-queue rules report inapplicable under an overlay" }
  - { mechanism: policy-merge, answer: n/a, why: "src/policy.rs only gains a test of the own-policy refusal (A5); merging is unchanged" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/verify.rs (A7)" }
  - { mechanism: commands, answer: works, proof: "test:src/main.rs (A6)" }
guarantee:
  - { holds: "an overlay run reports only what the code earns; rules needing state beside the policy say inapplicable per rule with a reason", not_against: "the missing ratchet: an overlay run carries none and says so" }
cut:
  - "a hand-written list of inapplicable rule ids: refused (S7)"
exit: "an overlay run over a repository with no `.software-factory/` reports only the findings its code earns; each rule whose state travels with the policy says so with its reason; vendored, it reports the same; each owed refusal carries a test"
---
# An overlay run says what it cannot know

`sf check --policy` shipped without inapplicability, so rules reading state that travels with the policy read its absence in the target as violations: eighty-odd findings on code that earned none.
