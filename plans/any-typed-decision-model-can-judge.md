# Any typed decision model can judge

#63 wires the claim-support rule to TypeSafe's Jev. What makes
Jev fit a gate is not the vendor but the shape of the answer: state plus named
questions (`choice`, `score`, `noul`) in, one label per question with a
probability out. Other models answer the same way, and several of them run
locally with open weights:

| Backend | Where it runs | Weights | Notes |
| :-- | :-- | :-- | :-- |
| [Jev](https://docs.typesafe.ai/api.md) | hosted API, paid | closed | `POST /v1/systemone`, versions like `jev-1.13.0` |
| [CLM](https://github.com/Contrastive-LM/CLM) | local, NVIDIA GPU with vLLM | Apache-2.0 | `CLM-v0.1-8B`, `temperature: 0` for repeatable answers |
| [laya-mlx](https://github.com/mizorewww/laya-mlx) | local, Apple Silicon | Apache-2.0 | about 421M parameters, same answer across repeated calls |
| [Qwen-2.5-1B-RLCD](https://huggingface.co/harshatheg/Qwen-2.5-1B-RLCD) | local, MLX or transformers | Apache-2.0 | returns `{value, prob}` per field with top choices, so it needs a thin adapter |

A deterministic tool that recommends one paid API has quietly become a tool
that needs an account to go green. The point of this plan is the opposite: any
backend that speaks the shape can judge, the tests of this repository run on a
backend that costs nothing, and nobody spends tokens to keep a build green.

## Precondition

The protocol and its adapters land with the split of #63: one pull request for
the rule itself, one for the request and response format with a Jev adapter, at
least one local adapter, and a fake backend. This plan starts from there.

## What changes

- The request and response format is written down once, as a schema this
  repository owns, and `sf judge` (see [a verdict is evidence](a-verdict-is-evidence.md))
  validates every response against it. A response with an unknown label, a
  missing question or a probability outside `[0, 1]` is a backend failure and
  a finding, never a verdict.
- A fake backend ships in the test tree. It answers from a table keyed on the
  request, so every end to end case below is exact, offline and free.
- A conformance suite replays recorded responses from each backend in the table
  above through its adapter and requires the same normalized answer. Recording
  a new response is a deliberate act; replaying it is a unit test.
- A live job, outside the gate, runs the same suite against each backend that
  can run where the job runs (Jev with a secret, laya-mlx and the Qwen adapter
  on a macOS runner). It reports drift; it never blocks a merge.

## End to end cases

All against the fake backend, all in one test file, all offline:

1. A supported claim is green, and the output names the claim, the label and
   the probability.
2. A report that says `passed` without the effect the claim promises is a
   finding that points at the claim's file and line.
3. The right label below the floor is a finding with its own message.
4. A strengthened claim ("handles retries" to "handles retries under
   partition") has no matching verdict, and `sf check` is red until it is
   judged again.
5. A resealed gate with a different report invalidates the verdicts that cite
   it.
6. Several claims with one bad one report only that one.
7. A backend that is unreachable, returns malformed JSON or an unknown label is
   a finding, never a skip.
8. An always-agreeing backend is refused by `sf verify` through the twin.
9. A lowered floor in the policy is an `L2.POLICY_ONLY_TIGHTENS` finding.
10. Two runs of `sf check` produce byte-identical output, and a sealed
    repository is green with the network disabled.

## Acceptance criteria

- [ ] Every response is validated against the schema before it becomes a
      verdict, and each malformed kind is a distinct finding
      (proof: test:src/judge.rs)
- [ ] The ten cases above pass against the fake backend with no network
      (proof: test:tests/judge_e2e.rs)
- [ ] Recorded responses from Jev, CLM, laya-mlx and Qwen-2.5-1B-RLCD
      normalize to the same answer through their adapters
      (proof: test:tests/judge_conformance.rs)
- [ ] The live drift job runs each backend it can reach and does not gate
      merges
      (proof: deferred:the job's runners and secrets are decided when the
      adapters exist)

**Exit condition:** this repository's own tests prove the rule end to end with
no account, no key and no tokens spent, and switching a consumer from Jev to a
local model is a change to one backend command with no change to the question
file or the sealed verdict format.
