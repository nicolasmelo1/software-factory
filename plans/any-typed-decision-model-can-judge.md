---
sources:
  - { id: S1, kind: ticket, ref: "#63", said: "wires the claim-support rule to TypeSafe's Jev; what fits a gate is the answer's shape: state plus named questions (choice, score, noul) in, one label per question with a probability out" }
  - { id: S2, kind: reference, ref: "https://docs.typesafe.ai/api.md", said: "Jev: hosted paid API, closed weights, `POST /v1/systemone`, versions like `jev-1.13.0`" }
  - { id: S3, kind: reference, ref: "https://github.com/Contrastive-LM/CLM", said: "CLM: local, NVIDIA GPU with vLLM, Apache-2.0, `CLM-v0.1-8B`, `temperature: 0` for repeatable answers" }
  - { id: S4, kind: reference, ref: "https://github.com/mizorewww/laya-mlx", said: "laya-mlx: local, Apple Silicon, Apache-2.0, about 421M parameters, same answer across repeated calls" }
  - { id: S5, kind: reference, ref: "https://huggingface.co/harshatheg/Qwen-2.5-1B-RLCD", said: "Qwen-2.5-1B-RLCD: local, MLX or transformers, Apache-2.0; returns `{value, prob}` per field with top choices, so needs a thin adapter" }
  - { id: S6, kind: decision, said: "a deterministic tool must not need an account to go green: any backend speaking the shape judges, this repository's tests run on a free backend, nobody spends tokens on a green build" }
  - { id: S7, kind: ticket, ref: "#63", said: "precondition: #63 splits into a PR for the rule and one for the request/response format with a Jev adapter, at least one local adapter, and a fake backend; this plan starts there" }
  - { id: S8, kind: decision, said: "the format is a schema this repository owns; sf judge validates every response; an unknown label, missing question or probability outside [0, 1] is a backend failure and a finding, never a verdict" }
  - { id: S9, kind: reference, ref: "plans/a-verdict-is-evidence.md", said: "`sf judge` is the command that asks the backend" }
  - { id: S10, kind: decision, said: "a fake backend in the test tree answers from a table keyed on the request, so every end-to-end case is exact, offline and free, all in one test file" }
  - { id: S11, kind: decision, said: "e2e 1-3: a supported claim is green, output naming claim, label, probability; a `passed` report lacking the promised effect is a finding at the claim's file and line; right label below the floor has its own message" }
  - { id: S12, kind: decision, said: "e2e 4-5: a strengthened claim ('handles retries' to 'handles retries under partition') has no matching verdict, red until rejudged; a resealed gate with a different report invalidates verdicts citing it" }
  - { id: S13, kind: decision, said: "e2e 6-8: several claims with one bad report only that one; an unreachable backend, malformed JSON or unknown label is a finding, never a skip; an always-agreeing backend is refused by sf verify through the twin" }
  - { id: S14, kind: decision, said: "e2e 9-10: a lowered floor is an L2.POLICY_ONLY_TIGHTENS finding; two sf check runs give byte-identical output, and a sealed repository is green with the network disabled" }
  - { id: S15, kind: decision, said: "a conformance suite replays each backend's recorded responses through its adapter and requires the same normalized answer; recording is deliberate, replay is a unit test" }
  - { id: S16, kind: decision, said: "a live job outside the gate runs the suite against each reachable backend (Jev with a secret, laya-mlx and the Qwen adapter on a macOS runner); it reports drift, never blocks a merge" }
touches:
  - src/judge.rs
  - tests/judge_e2e.rs
  - tests/judge_conformance.rs
  - .github/workflows/**
criteria:
  - { id: A1, statement: "every response is validated against the schema before it becomes a verdict, and each malformed kind is a distinct finding", proof: "test:src/judge.rs", sources: [S1, S7, S8, S9] }
  - { id: A2, statement: "the ten end-to-end cases pass against the fake backend with no network", proof: "test:tests/judge_e2e.rs", sources: [S6, S10, S11, S12, S13, S14] }
  - { id: A3, statement: "recorded responses from Jev, CLM, laya-mlx and Qwen-2.5-1B-RLCD normalize to the same answer through their adapters", proof: "test:tests/judge_conformance.rs", sources: [S2, S3, S4, S5, S15] }
  - { id: A4, statement: "the live drift job runs each backend it can reach and does not gate merges", proof: "deferred:the job's runners and secrets are decided when the adapters exist", sources: [S16] }
failures:
  - { step: "response validation", when: "unknown label, missing question, probability outside [0, 1], malformed JSON or unreachable backend", goes_to: "backend failure finding; never a verdict, never a skip" }
  - { step: "live drift job", when: "a backend's answer drifts", goes_to: "reported; the merge is not blocked" }
cut:
  - "a backend that needs a paid account for the build to go green: refused (S6)"
  - "the live job gating merges: refused (S16)"
exit: "this repository's own tests prove the rule end to end with no account, key or tokens spent, and switching a consumer from Jev to a local model changes one backend command, not the question file or sealed verdict format"
---
# Any typed decision model can judge

#63 recommends one paid API, so a deterministic tool quietly needs an account to go green. Any backend that answers in the typed decision shape should be able to judge.
