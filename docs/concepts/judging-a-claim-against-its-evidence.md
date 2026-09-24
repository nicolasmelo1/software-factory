# Judging a claim against its evidence

Some claims a repository makes about itself are not decidable by a query.

`L4.CLAIM_CITES_ITS_EVIDENCE` requires every marked promise to name a declared
gate, and `L3.GATE_HAS_FRESH_EVIDENCE` expires that join the moment the
implementation moves. Neither of them reads the sentence. A report can say
`passed` without ever containing the effect the sentence promises.

`L4.CLAIM_IS_SUPPORTED_BY_ITS_EVIDENCE` is the rule that closes that gap, and
it is the only rule in the catalog that ships with no check of its own: it runs
a command the repository owns. This page is that command. Every file in it is
complete — copying the page top to bottom takes the rule from off to green.

## From zero to green

Before step 1 you need `sf` adopted in the repository, Node 18 or newer, and a
[TypeSafe](https://typesafe.ai) account. Everything else is created below.

### 1. Configure the credential

Create an API key in your TypeSafe account and export it:

```sh
export TYPESAFE_API_KEY=tsk_...   # never committed; in CI, a secret
```

Optional, and only if you were told otherwise:

```sh
export TYPESAFE_BASE_URL=https://api.typesafe.ai   # this is the default
```

Leave `TYPESAFE_BASE_URL` alone unless your account, gateway or private
deployment gave you a different URL — regional endpoints included. Whatever
URL you were told to call, that value is the one to export.

A missing key is a finding, not a skip: the judge below exits nonzero when it
cannot run, because an oracle that silently passes reports green for the one
class of defect nobody else is looking for.

### 2. Create the judge — `tools/claim-judge.mjs`

The whole file. No dependencies, no `npm install` — Node 18's `fetch` is the
only client:

```js
#!/usr/bin/env node
// The oracle L4.CLAIM_IS_SUPPORTED_BY_ITS_EVIDENCE runs.
//
// Exit codes are the whole contract:
//   0  expected question, expected choice, confidence at or above the floor
//   1  the finding — wrong choice, or the right choice below the floor
//   2  usage, request, cache or API failure; also a finding, never a skip

import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";

const die = (code, message) => {
  console.error(message);
  process.exit(code);
};

// --- the command line -------------------------------------------------------

const flag = (name) => {
  const at = process.argv.indexOf(`--${name}`);
  return at >= 0 ? process.argv[at + 1] : undefined;
};

const requestPath = flag("request");
const cachePath = flag("cache");
const expected = flag("expect");
const floor = Number(flag("min-confidence") ?? 0.8);

if (!requestPath || !cachePath || !expected)
  die(2, "usage: claim-judge.mjs --request FILE --expect name=label --min-confidence N --cache FILE");

const [question, wanted] = expected.split("=");
if (!question || !wanted)
  die(2, `--expect must be name=label, got "${expected}"`);

// --- the request ------------------------------------------------------------

let request;
try {
  request = JSON.parse(readFileSync(requestPath, "utf8"));
} catch (error) {
  die(2, `cannot read ${requestPath}: ${error.message}`);
}

if (!request.state?.claim || !request.questions?.[question])
  die(2, `${requestPath} needs state.claim and a question named "${question}"`);

// --- the cache: everything that changes the verdict is the key --------------

const model = request.model ?? "jev-latest";
const key = createHash("sha256")
  .update(JSON.stringify({ model, state: request.state, questions: request.questions }))
  .digest("hex");

let cache = {};
if (existsSync(cachePath)) {
  try {
    cache = JSON.parse(readFileSync(cachePath, "utf8"));
  } catch (error) {
    die(2, `cannot read ${cachePath}: ${error.message}`);
  }
}

// --- ask, unless this exact question was already answered -------------------

let answer = cache[key];
if (!answer) {
  const apiKey = process.env.TYPESAFE_API_KEY;
  if (!apiKey)
    die(2, "TYPESAFE_API_KEY is not set — an oracle nobody can run is itself the finding");

  const base = process.env.TYPESAFE_BASE_URL ?? "https://api.typesafe.ai";
  let response;
  try {
    response = await fetch(`${base}/v1/systemone`, {
      method: "POST",
      headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
      body: JSON.stringify({ model, state: request.state, questions: request.questions }),
    });
  } catch (error) {
    die(2, `cannot reach ${base}: ${error.message}`);
  }
  if (!response.ok)
    die(2, `the API answered ${response.status}: ${(await response.text()).slice(0, 400)}`);

  const judged = await response.json();
  answer = judged.answers?.[question];
  if (!answer?.choice)
    die(2, `the API returned no choice for "${question}"`);

  cache[key] = answer;
  try {
    writeFileSync(cachePath, `${JSON.stringify(cache, null, 2)}\n`);
  } catch (error) {
    die(2, `cannot write ${cachePath}: ${error.message}`);
  }
}

// --- the verdict ------------------------------------------------------------

const confidence = answer.confidence ?? 0;
const claim = request.state.claim;

if (answer.choice !== wanted) {
  console.error(`claim not supported: "${claim}"`);
  console.error(`judged ${answer.choice} at ${confidence.toFixed(2)}, expected ${wanted}`);
  die(1, "the evidence does not support the sentence");
}
if (confidence < floor) {
  console.error(`claim supported without conviction: "${claim}"`);
  console.error(`${wanted} at ${confidence.toFixed(2)}, below the floor ${floor}`);
  die(1, "right choice, no conviction — usually a badly posed question or thin evidence");
}

console.log(`claim supported: "${claim}" — ${wanted} at ${confidence.toFixed(2)} (floor ${floor})`);
```

### 3. Create the question — `gates/jev/claims.json`

The judge answers only what this file asks, so the question is committed and
reviewed like any other check. Two strings in it are yours: the `claim`, and
the `evidence`, pasted verbatim from the gate's run report
(`.software-factory/evidence/<gate>-run.json`):

```json
{
  "model": "jev-latest",
  "state": {
    "claim": "Adopting the factory in an existing repository ends green.",
    "evidence": "…the run report of the adoption gate, verbatim…"
  },
  "questions": {
    "relation": {
      "type": "choice",
      "instructions": "Does the evidence support the claim? Judge only what the evidence shows.",
      "criteria": {
        "supports": "the evidence contains the effect the claim promises",
        "contradicts": "the evidence shows the opposite of the claim",
        "says_nothing": "the evidence is silent about what the claim promises"
      }
    }
  }
}
```

Two rules of thumb: one narrow question, and criteria that genuinely separate
the options — if two choices can both be argued for the same input, the verdict
is noise with a confidence number attached. And nothing secret in `state`:
everything in it leaves the machine.

### 4. Enable the rule

```yaml
# .software-factory/policy.yaml
  L4.CLAIM_IS_SUPPORTED_BY_ITS_EVIDENCE:
    enabled: true
    options:
      run: >-
        node tools/claim-judge.mjs
        --request gates/jev/claims.json
        --expect relation=supports
        --min-confidence 0.8
        --cache gates/jev/claims.cache.json
```

Then re-lock, because the policy hash covers the command:

```sh
sf lock
```

`--expect` and `--min-confidence` stay on the command line on purpose: the
policy diff shows what the check accepts, so loosening the threshold is a
change to a locked file instead of a change to a JSON blob nobody re-reads.

### 5. Run it

```sh
sf check --allow-commands
```

The first run asks the API once and writes `gates/jev/claims.cache.json`;
every run after that replays the cached verdict — exact, offline, free. Commit
the cache. A rewritten question hashes to a new key and calls out again, which
is correct: it is a different check, and the old verdict was about something
else.

### 6. Prove the oracle can fail

An oracle only ever seen passing may be approving anything. Copy
`claims.json` to `claims.twin.json` and swap the criteria of `supports` and
`says_nothing` — same claim, same evidence, the definition of passing turned
upside down — then demand a pass anyway:

```sh
node tools/claim-judge.mjs --request gates/jev/claims.twin.json \
  --expect relation=supports --min-confidence 0.8 --cache gates/jev/claims.cache.json
echo "exit $?"
```

The real request must exit `0` and the twin must exit `1`. Wrong on either
side is a broken judge. This is the same argument that makes a mutation
fixture mandatory for every other rule, applied to the one check `sf verify`
cannot reason about from a query.

## The contract the judge keeps

| exit | meaning |
| :-- | :-- |
| `0` | every expected question landed on the expected choice at or above the floor |
| `1` | wrong choice, or right choice below the confidence floor — the finding |
| `2` | usage, request, cache or API failure — also a finding, not a skip |

The passing line prints the claim, the choice and the confidence too. `sf`
surfaces the command's output in the finding, and "which claim, judged how
confidently" is the whole diagnostic.

## Why it is shaped this way

**The deterministic part runs first.** Finding the claim markers, resolving
the gate each one names, and loading that gate's report are string operations —
the two rules this page opened with already keep that join honest. The judge
sees only the residue nothing else can decide: whether *this* evidence
supports *this* sentence. A judgment placed where a `grep` belongs costs
money, can flake, and teaches people to distrust the rule.

**A low verdict is a finding.** `0.8` is a reasonable starting floor, and the
failure line distinguishes "landed on the wrong choice" — a bad claim — from
"landed on the right one without conviction" — usually a badly posed question
or thin evidence. They call for different fixes. Lowering the floor to go
green is a change `L2.POLICY_ONLY_TIGHTENS` will notice and refuse.

**Verdicts are cached and committed.** An uncached request makes the check
nondeterministic, network-dependent and priced per run — none of which a gate
can be. The cache key covers the model, the state and the questions, so a
changed question is a new key and a fresh judgment, never a stale yes.

**The oracle is proven on both sides.** Step 6 exists because an oracle that
can only pass is indistinguishable from one that works until the day it
approves something false.

## Why the catalog ships no judge

The catalog names no code generator either, for the same reason. A judgment
that costs money or needs a credential belongs to the repository running it:
the key is theirs, the bill is theirs, and the verdict has to be reviewable in
their diff rather than produced inside a binary they installed from a tag.
Shipping a default would also fix the wording of the question for everyone —
and with this kind of check, the wording *is* the check.

What the rule adds over writing the same command as a plain CI step is
everything around it: the reason printed where it fails, a mutation proving it
still fails when it should, a ratchet for the debt already there, and a policy
`L2.POLICY_ONLY_TIGHTENS` will not let anyone quietly loosen.

## What it cannot do

A calibrated judgment is not a proof. Confidence summarises the probability
distribution over *this* question, not the correctness of the work, so a
consequential low-confidence verdict belongs in front of a person rather than
under a lowered floor. And the judge only knows what the evidence file says:
it cannot tell you the test that produced it was meaningful. That is
`sf verify`'s job, one layer down.

## Why this repository has it switched off

No judge has been written here yet. Enabling the rule with nothing in `run`
would be a rule lying about its own coverage, which is exactly what
`L5.NO_INERT_RULE` exists to refuse — so the decision is written down in
[`docs/rules.md`](../rules.md) and the rule stays off until the oracle exists.
The marked claims themselves are already policed by the two rules this page
opened with. What neither of them can do is read the sentence.
