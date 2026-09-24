# A verdict is evidence

the claim-support rule asks a model whether a gate's report
supports the sentence a claim marker sits on. As first proposed in #63, the
command that asks does everything itself: it holds the claim and the evidence
in a hand-written request file, hashes it, calls out on a cache miss and writes
the cache wherever it runs. That leaves three holes, all verified against the
proposal:

- The evidence is pasted into the request by hand. Reseal the gate with a
  different report and the request, its hash and its cached verdict stay the
  same, so the check stays green about a report that no longer exists.
- A cache miss in CI calls the network and writes a cache nobody commits, so
  every CI run is priced, can flake, and can disagree with the last one.
- One request holds one claim, while the rule promises every marked claim.

None of that is a judgment. Finding the markers, resolving the gate each one
names, reading that gate's report and deciding whether a verdict is still about
the same inputs are string operations, and this tool already does the first two
in `src/checks/cadence.rs`. They belong in `sf`, where they are deterministic
and tested, and the model should see only what nothing else can decide.

## What changes

- A new subcommand, `sf judge`, is the only thing that ever asks a model. For
  every marked claim it resolves the gate, reads the report that gate's
  evidence manifest points at, and builds one request per claim in the typed
  decision shape (state plus named questions, see
  [any typed decision model can judge](any-typed-decision-model-can-judge.md)).
- The request goes to a backend command over stdin and comes back over stdout.
  `sf` holds no HTTP client and no model runtime: the backend is the
  repository's own, which is the reason #63 gave for shipping no judge, and it
  stays true.
- Each verdict is sealed beside the gate's evidence, keyed on the digest of the
  claim sentence, the report, the question file and the model identity the
  backend reports (not an alias like `jev-latest`). This is the same object an
  L3 manifest already is: produced once, on purpose, and verified forever
  after.
- `sf check` never calls a backend. It recomputes every key and compares it
  with the sealed one. A claim with no verdict, or with a verdict about
  different inputs, is a finding that names the claim and says which input
  moved. That is the frozen mode #63 lacked, and it is the only mode `check`
  has.
- The rule stops being a `kind: command` that runs a judge and becomes a check
  over sealed verdicts. `--allow-commands` is then needed by `sf judge`, not by
  `sf check`.

## What this does not claim

It does not make the verdict right. A sealed verdict is exactly as good as the
model and the question that produced it. What it guarantees is narrower and
checkable: the build never goes green on a verdict about a sentence or a report
that has since changed, and the build never depends on a network it does not
control.

## Acceptance criteria

- [ ] `sf judge` builds one request per marked claim, with the gate's actual
      report as evidence, and none of it is written by hand
      (proof: test:src/judge.rs)
- [ ] Resealing a gate with a different report invalidates every verdict about
      claims that cite it, and `sf check` names the claim and the moved input
      (proof: test:src/checks/judgment.rs)
- [ ] Editing a claim sentence invalidates its verdict the same way
      (proof: test:src/checks/judgment.rs)
- [ ] The verdict key carries the model identity the backend returned, so a
      backend that upgrades behind an alias invalidates rather than silently
      agreeing
      (proof: test:src/judge.rs)
- [ ] `sf check` performs no process spawn and no network access for this
      rule, and is green offline on a sealed repository
      (proof: test:tests/judge_e2e.rs)
- [ ] `sf judge` without `--allow-commands` refuses, and says so, rather than
      reporting anything as judged
      (proof: test:src/judge.rs)

**Exit condition:** a repository with marked claims runs `sf judge` once,
commits the sealed verdicts, and from then on `sf check` is deterministic and
offline for this rule, and goes red the moment a claim sentence or the report
it cites changes without being judged again.
