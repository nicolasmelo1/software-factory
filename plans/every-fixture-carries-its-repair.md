---
sources:
  - { id: S1, kind: code, ref: "src/escapes.rs", said: "shipped in 703f899: records attempts per finding key, minimises an escape to the key-flipping hunks, classes policy/ratchet/lock hunks as weakening, grows the trail past the attempt ceiling; 21 tests" }
  - { id: S2, kind: code, ref: "src/fixtures.rs", said: "`Fixture` has four fields: rule, two policy fragments, files; none is a repair. Forty fixtures, zero repairs; sf verify asserts the trip, never the clear" }
  - { id: S3, kind: friction, said: "retrieval's last resort, the rule's own worked example, resolves to nothing for every rule, so the log stays empty until someone here is stuck on that rule" }
  - { id: S4, kind: reference, ref: "34cd141^:plans/gates-record-how-they-were-unblocked.md", said: "a log that only accumulates learns from whoever was stuck first; every check has a mutation proving it fires and a repair proving it clears" }
  - { id: S5, kind: reference, ref: "34cd141^:plans/gates-record-how-they-were-unblocked.md", said: "promised `assertion:escape.breaks_a_real_loop` (a model looping on a real finding gets out with the log); no gate requires it and no manifest carries it" }
  - { id: S6, kind: decision, said: "an assertion that exists nowhere cannot go green: it is a required gate assertion with a harness, or not a criterion; whoever picks this up decides; repair fixtures are a loop test's corpus" }
touches:
  - src/fixtures.rs
  - src/verify.rs
  - src/escapes.rs
criteria:
  - { id: A1, statement: "`Fixture` carries a repair, and `sf verify` fails both when the mutation does not trip the rule and when the repair does not clear it", proof: "test:src/verify.rs", sources: [S2, S4] }
  - { id: A2, statement: "every fixture carries one, in every language the fixture claims, so no rule ships with a before and no after", proof: "test:src/fixtures.rs", sources: [S2, S4] }
  - { id: A3, statement: "retrieval falls back to the rule's repair when no local escape ranks, so a rule nobody has been stuck on still renders a worked example", proof: "test:src/escapes.rs", sources: [S1, S3] }
  - { id: A4, statement: "whether 'a model looping on a real finding gets out' is a required gate assertion or not a criterion is decided before the repairs land", proof: "unspecified:it asks what evidence a gate should require, and no check answers that", sources: [S5, S6] }
  - { id: A5, statement: "a recurring escape is promoted into the rule's `fix` prose and deleted", proof: "deferred:needs the log to have run long enough to have produced a recurring entry", sources: [S1, S4] }
crosses:
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/verify.rs (A1); test:src/fixtures.rs (A2)" }
  - { mechanism: locks-and-ratchet, answer: n/a, why: "escapes.rs gains only a retrieval fallback to the rule's repair; nothing writes digests or frozen violations" }
exit: "`sf verify` reports, for every enabled rule, that its mutation trips it and its repair clears it, and a finding on a rule nobody here was ever stuck on renders a worked example from the binary, not a working tree"
---
# Every fixture carries its repair

The escape log shipped without its cold start: no fixture carries a repair, so retrieval's last resort resolves to nothing and the log is empty until someone has personally been stuck on a rule.
