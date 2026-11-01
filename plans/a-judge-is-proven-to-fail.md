---
sources:
  - { id: S1, kind: ticket, ref: "#63", said: "the claim-support rule's mutation sets `run: \"exit 1\"`: it proves sf reports a failing command, not the judge; a judge answering `supports` to everything passes every run forever" }
  - { id: S2, kind: ticket, ref: "#63", said: "the proposal asks for a twin request with the criteria swapped, run by hand; by hand means never, the argument for mutation fixtures" }
  - { id: S3, kind: reference, ref: "plans/a-hazard-tool-is-proven-to-fail.md", said: "same shape: the check delegates, and nothing asks whether the delegate can say no" }
  - { id: S4, kind: decision, said: "sf verify builds the twin: per expected question, the same state with expected and failing labels' criteria swapped; the real request lands on the expected label, the twin away from it" }
  - { id: S5, kind: decision, said: "a backend passing both is refused as unable to fail; failing both, as unable to pass" }
  - { id: S6, kind: decision, said: "the twin runs against a planted claim with a planted report, needing no real claim, so it works on the first day" }
  - { id: S7, kind: code, ref: "src/checks/tightening.rs", said: "L2.POLICY_ONLY_TIGHTENS counts exclude, scope, max, forbidden_* and cannot see into a command; a floor lowered 0.8 to 0.1 only trips L2.FACTORY_CONFIG_IS_LOCKED, which sf lock clears" }
  - { id: S8, kind: decision, said: "`expect` and `min_confidence` become structured rule options, not flags in a `run` string, so a lower floor or wider accepted labels is a loosening refused" }
  - { id: S9, kind: ticket, ref: "#63", said: "wrong label is a bad claim; right label below the floor is usually a badly posed question or thin evidence; different fixes, so different messages" }
touches:
  - src/judge.rs
  - src/checks/judgment.rs
  - src/checks/tightening.rs
  - src/verify.rs
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "`sf verify` refuses a backend that answers the expected label for both the request and its twin", proof: "test:src/judge.rs", sources: [S1, S2, S3, S4, S5, S6] }
  - { id: A2, statement: "`sf verify` refuses a backend that answers against the expected label for both", proof: "test:src/judge.rs", sources: [S4, S5] }
  - { id: A3, statement: "the rule's mutation fixture is an always-agreeing backend, and `sf verify` goes red on it", proof: "test:src/fixtures.rs", sources: [S1] }
  - { id: A4, statement: "lowering `min_confidence` or adding a label to `expect` is reported by `L2.POLICY_ONLY_TIGHTENS`; raising or narrowing them is not", proof: "test:src/checks/tightening.rs", sources: [S7, S8] }
  - { id: A5, statement: "a wrong label and a right label below the floor produce distinct finding messages", proof: "test:src/checks/judgment.rs", sources: [S9] }
crosses:
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A3)" }
  - { mechanism: tightening, answer: works, proof: "test:src/checks/tightening.rs (A4)" }
guarantee:
  - { holds: "the backend responds to the criteria: it lands on the expected label and away from it on the twin", not_against: "badly written criteria, or overlapping options passing both sides by accident" }
cut:
  - "running the twin by hand: refused (S2)"
  - "proving criteria are well written: stays review work on a committed file"
exit: "a repository whose judge approves everything is told so by `sf verify` before any claim is judged, and relaxing what the judge accepts is a policy change the L2 rules refuse rather than a string edit `sf lock` absorbs"
---
# A judge is proven to fail

The claim-support rule's mutation proves only that sf reports a failing command; a judge that agrees with everything passes forever, and loosening what it accepts is a string edit `sf lock` absorbs.
