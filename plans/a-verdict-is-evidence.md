---
sources:
  - { id: S1, kind: ticket, ref: "#63", said: "the claim-support rule asks a model whether a gate's report supports a claim's sentence; its command holds claim and evidence in a hand-written request, hashes it, calls out on a cache miss, writes the cache anywhere" }
  - { id: S2, kind: ticket, ref: "#63", said: "evidence is pasted by hand: reseal with a different report and request, hash and cached verdict stay the same, green about a report that no longer exists" }
  - { id: S3, kind: ticket, ref: "#63", said: "a CI cache miss calls the network and writes a cache nobody commits: every run is priced, can flake, and can disagree with the last" }
  - { id: S4, kind: ticket, ref: "#63", said: "one request holds one claim, while the rule promises every marked claim" }
  - { id: S5, kind: code, ref: "src/checks/cadence.rs", said: "already finds markers and resolves the gate each names; reading the report and judging input sameness are string operations that belong in sf; the model sees only what nothing else decides" }
  - { id: S6, kind: decision, said: "`sf judge` alone asks a model: per marked claim it resolves the gate, reads the report the evidence manifest points at, and builds one request in the typed decision shape" }
  - { id: S7, kind: reference, ref: "plans/any-typed-decision-model-can-judge.md", said: "the typed decision shape: state plus named questions" }
  - { id: S8, kind: decision, said: "requests go to a backend command over stdin/stdout; sf holds no HTTP client or model runtime; the backend is the repository's own, #63's reason for shipping no judge" }
  - { id: S9, kind: decision, said: "each verdict is sealed beside the gate's evidence, keyed on digests of claim sentence, report, question file and backend-reported model identity (not an alias like jev-latest), like an L3 manifest" }
  - { id: S10, kind: decision, said: "`sf check` never calls a backend: it recomputes every key against the sealed one; no verdict or different inputs is a finding naming the claim and the moved input. Check's only mode" }
  - { id: S11, kind: decision, said: "the rule stops being `kind: command` and becomes a check over sealed verdicts; `--allow-commands` is needed by `sf judge`, not `sf check`" }
touches:
  - src/judge.rs
  - src/checks/judgment.rs
  - src/checks/mod.rs
  - src/catalog.rs
  - src/main.rs
  - tests/judge_e2e.rs
criteria:
  - { id: A1, statement: "`sf judge` builds one request per marked claim, with the gate's actual report as evidence, none of it written by hand", proof: "test:src/judge.rs", sources: [S1, S4, S5, S6, S7] }
  - { id: A2, statement: "resealing a gate with a different report invalidates every verdict about claims citing it, and `sf check` names the claim and the moved input", proof: "test:src/checks/judgment.rs", sources: [S2, S9, S10] }
  - { id: A3, statement: "editing a claim sentence invalidates its verdict the same way", proof: "test:src/checks/judgment.rs", sources: [S9, S10] }
  - { id: A4, statement: "the verdict key carries the model identity the backend returned, so a backend upgrading behind an alias invalidates rather than silently agreeing", proof: "test:src/judge.rs", sources: [S9] }
  - { id: A5, statement: "`sf check` performs no process spawn and no network access for this rule, and is green offline on a sealed repository", proof: "test:tests/judge_e2e.rs", sources: [S3, S8, S10, S11] }
  - { id: A6, statement: "`sf judge` without `--allow-commands` refuses, and says so, rather than reporting anything as judged", proof: "test:src/judge.rs", sources: [S11] }
crosses:
  - { mechanism: commands, answer: works, proof: "test:src/judge.rs (A6); test:tests/judge_e2e.rs (A5)" }
  - { mechanism: catalog, answer: works, proof: "test:src/checks/judgment.rs (A2)" }
  - { mechanism: evidence, answer: works, proof: "test:src/checks/judgment.rs (A2): a resealed report invalidates its verdicts" }
  - { mechanism: inertness, answer: n/a, why: "a repository with no marked claim gives the check nothing to compare; the plan adds no inert path" }
  - { mechanism: overlay, answer: refused, by: "verdicts are sealed beside the gate's evidence, repo-local state an overlay does not carry, as the Evidence kind is in src/checks/mod.rs" }
guarantee:
  - { holds: "the build never goes green on a verdict about a since-changed sentence or report, and never depends on a network it does not control", not_against: "a wrong verdict: it is as good as the model and the question" }
failures:
  - { step: "sf judge", when: "`--allow-commands` is absent", goes_to: "refuses and says so; nothing reported as judged" }
  - { step: "sf check", when: "a claim has no verdict or one about different inputs", goes_to: "finding naming the claim and which input moved" }
cut:
  - "an HTTP client or model runtime inside sf: refused (S8)"
  - "sf check calling a backend or writing a cache: refused (S10)"
  - "making the verdict right"
exit: "a repository with marked claims runs `sf judge` once and commits the sealed verdicts; then `sf check` is deterministic and offline for this rule, and goes red the moment a claim sentence or its cited report changes unjudged"
---
# A verdict is evidence

As proposed in #63, the claim-support command pastes evidence by hand, calls the network on a CI cache miss, and judges one claim per request, so it stays green about reports that no longer exist.
