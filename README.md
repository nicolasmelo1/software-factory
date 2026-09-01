# software factory

**A method for building software with agents, packaged as a single binary that runs against any repository.**

---

## Why this exists

Generating code stopped being the hard part. Generating the *right* code — code
that fits the requirement, respects the boundary, and can be trusted without a
senior engineer reconstructing the whole chain of intent behind it — is still
the bottleneck. Teams solve it by hand-rolling an internal harness of prompts,
conventions, review rituals and glue scripts, and then discover the harness has
become a second codebase nobody budgeted for.

`sf` is that harness, extracted from a working one, made language-neutral, and
reduced to a single method:

> **Every rule that matters is written twice — once as prose that says *why*,
> once as a check that *fails*. And every check has a mutation that proves it
> fires.**

It ships 34 rules across seven layers — from where an error type may be defined
to whether your CI still runs a vulnerability scanner — plus 4 rule templates an
interview fills in with your own package and directory names. It protects its
own configuration, so an agent cannot reach a green build by turning a rule off.

Everything else in this repository is a consequence of that sentence.

---

## Quickstart (5 minutes)

Works on any repository, in any language. Nothing to configure first, and it
will not change a line of your code.

### 1. Install — 1 min

Download the binary for your platform from the
[latest release](https://github.com/nicolasmelo1/software-factory/releases/latest)
— no toolchain, no compile:

```sh
# macOS on Apple silicon; swap the target for x86_64-apple-darwin
# or x86_64-unknown-linux-gnu
curl -fsSLO https://github.com/nicolasmelo1/software-factory/releases/latest/download/sf-aarch64-apple-darwin
curl -fsSLO https://github.com/nicolasmelo1/software-factory/releases/latest/download/sf-aarch64-apple-darwin.sha256
shasum -a 256 -c sf-aarch64-apple-darwin.sha256
chmod +x sf-aarch64-apple-darwin && mv sf-aarch64-apple-darwin ~/.local/bin/sf
```

Or build it, if you have cargo:

```sh
cargo install --git https://github.com/nicolasmelo1/software-factory --tag v0.2.0 --locked
```

**Pin the tag.** The rule catalog ships *inside* the binary, so tracking the
tip of `main` means an upstream commit can change what an enabled rule matches
and turn your build red with nothing in your repository having moved. `sf init`
writes the same pinned form into the CI workflow it generates.

`sf --version` reports the version *and* the catalog digest, because the
version number alone does not identify the rules:

```
sf 0.2.0 (catalog ddde87d963b7, 35 rules)
```

One static binary, no runtime, nothing to clone. Building it is a single
compile — around a minute cold, a few seconds if you already have the crates
cached. Everything below runs in well under a second, even on a large monorepo.

**If `sf` is then "command not found":** cargo installed it to `~/.cargo/bin`,
which is not on your `PATH`. The `rustup` installer adds that directory for
you; Homebrew, `apt` and Nix do not, and cargo says so in a warning at the end
of the install that is easy to scroll past. Fix it once:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && exec zsh
#                                               ~/.bashrc for bash
```

### 2. Set it up — 2 min

```sh
cd ~/code/your-project
sf skills
```

It asks where. There is no default on purpose — these skills are about *this*
repository's factory, and writing them into every project on the machine is a
decision you should make rather than one this makes for you:

```
Where should the skills go?

  1  /Users/you/code/your-project/.claude/skills   (this repository only)
  2  /Users/you/.claude/skills                     (every project on this machine)

[1]
```

Use `--project` or `--user` to skip the question, `--dir` for anywhere else. In
a script or CI it refuses to guess and tells you to pass one.

Then invoke the skill **by name**. It will not be reached for on its own:

```
/factory-init set up software-factory in this repo
```

The [`factory-init`](skills/factory-init/SKILL.md) skill takes it from there. It
reads your codebase first and answers whatever the code can answer itself —
which framework, where the client lives, whether there are already
`domain/`/`application/` directories. Then it asks you the rest in rounds, each
question numbered, each with a recommendation:

> ❓ **Q2** — **Architecture**: I can see `packages/*/domain/`, `application/`
> and `infrastructure/`, so this looks domain-driven. But
> `apps/api/src/routes/users.ts` opens a database connection directly. Is the
> layering the intent or the reality?
>
> ➡️ I'd answer `ddd` and freeze today's 40 violations with a six-month review
> date — but pick that deliberately, because it commits you to fixing them.

Layered or DDD, repositories or ORM-in-services, Zod or Pydantic and where
those schemas live, which packages the client must never import, whether
anything shares mutable state across threads. Those answers generate rules
carrying *your own* package names — not a generic starter set. See
[The interview](#the-interview) for the full decision tree.

<details>
<summary><b>No agent? One command instead.</b></summary>

```sh
cd ~/code/your-project
sf init --language typescript --layer L1,L4,L5,L6
git config core.hooksPath .githooks
```

Use `--language` for what you actually have: `python`, `typescript`, `go`,
`rust`, `ruby`, or several comma-separated. `--layer L1,L4,L5,L6` is the honest day-one
set — code quality, documentation cadence, the self-proving layer, and the
security tooling. You get the generic rules; the structural ones stay off until
you run the interview or write them yourself.

</details>

Either way, you end up with something like:

```
wrote 47 files:
  .software-factory/policy.yaml          # which rules are on
  docs/rules.md                          # why each one exists
  .allowed-root-files
  .github/workflows/software-factory.yml # CI, with the security tools wired in
  .githooks/pre-commit
  .software-factory/mutations/...        # a tiny broken repo per rule
  .software-factory/ratchet.yaml (106 existing violations frozen)
```

**Nothing is red yet.** Every violation that already existed was frozen with a
six-month review date. Only *new* ones fail — which is what makes this
adoptable on a codebase with years of history.

### 3. Prove the checks actually work — 10 sec

```sh
sf verify
```

```
✓ L1.NO_BLANKET_SUPPRESSION — 1 finding(s): Bare `# noqa` disables every rule on the line...
✓ L4.DOC_LINKS_RESOLVE — 1 finding(s): link target `../src/pricing/README.md` does not exist
...
17/17 enabled rules proven to fire
```

Every rule was just run against a repository built to violate it. This is the
step that separates enforcement from decoration: a check with a typo in it
passes silently forever and looks exactly like a check that works.

### 4. Watch it catch something — 30 sec

```sh
echo "# scratch notes" > NOTES.md
sf check
```

```
! medium L4.ROOT_FILES_ARE_DECLARED — New top-level files are declared before they appear
  why  `NOTES.md`, `PLAN.md`, `SUMMARY.md` at the repository root is the most
       recognizable signature of agent-authored work, and each one is context
       that belongs in a plan, a pull request body or a commit message —
       somewhere with a lifecycle.
  fix  Move the content to the plans directory or the pull request
       description. If the file really belongs at the root, add it to the
       allowlist in the same commit.
    NOTES.md — `NOTES.md` is at the repository root but not declared

1 findings across 1 rules (106 frozen by the ratchet)
```

```sh
rm NOTES.md   # green again
```

That output shape is the design. The failure message is the only documentation
an agent reliably reads, so every rule carries its reasoning to the point of
failure rather than leaving it in a file nobody opens.

### 5. Turn it on for real — 1 min

The generated workflow already runs `sf verify` then `sf check` on every pull
request, with the security tooling for your languages wired in. Commit it:

```sh
git add -A && git commit -m "chore: adopt software-factory"
```

Exit codes are hierarchical, so CI can tell the difference between "the tool
could not run" and "the repository has violations": `3` bootstrap failed,
`2` config error, `1` findings, `0` clean.

### If something goes wrong

| | |
|---|---|
| `sf check` red on `L5.NO_INERT_RULE` | A rule is switched on and pointed at nothing here. Give it a scope, or disable it in `policy.yaml` and write down why. |
| Too many findings to face | `sf ratchet --months 6` freezes today's state. It is debt with a due date, not permission. |
| A rule seems wrong | `sf explain <RULE>` gives the full reasoning. If it is genuinely wrong, change it — that is one of the four honest resolutions. |
| Want to see everything available | `sf catalog`, and `sf interview` for the decisions that generate rules. |
| `sf: command not found` after installing | `~/.cargo/bin` is not on your `PATH` — see step 1. |

---

---

## The seven layers

The numbers are identity, not sequence — grouped by what they are
about. The adoption order is below, and it is different.

34 rules, plus 4 templates the interview instantiates with your own names.

| | Layer | What it checks |
|---|---|---|
| **L0** | Shape | Where things live — error types, data access, entrypoints, layer boundaries |
| **L1** | Grain | How code reads — complexity ceiling, banned escape hatches, no blanket suppressions |
| **L2** | Contract | No drift from the source of truth — hash locks, dated exceptions, and the guardrail's own protection |
| **L3** | Effect | A real actor achieved the observable outcome, and the evidence has not gone stale |
| **L4** | Cadence | Docs, plans and rules stay attached to each other |
| **L5** | Meta | Every check is proven to fire, and none is enabled but inert |
| **L6** | Hazard | The defect classes this repository hunts — vulnerabilities, secrets, dead code, races, deadlock shapes |

Run `sf catalog` for the rules, `sf explain <RULE>` for the reasoning behind any
one of them.

### Adopt in this order

**Day one: L1, L4, L5.** This is `sf init`'s default, and it is a deliberate
recommendation rather than a shortcut. L1 costs an hour. L4 costs three
markdown files. L5 is what makes either of them mean anything.

**L0 after the third occurrence of a pattern.** Cementing a shape you have seen
twice is how you cement the wrong one. Wait until the repetition tells you what
the shape actually is.

**L6 as soon as you have CI** — `sf init --layer L1,L4,L5,L6` writes the steps
for you. It is the cheapest large win here: the tools already exist and are
better than anything this could contain, and what actually rots is whether they
are still wired in.

**L2's guardrail lock immediately, the rest when a second surface derives from
a first** — a generated client, a schema and its migrations, a design token and
its stylesheet.

**L0 after the third occurrence of a pattern.** Cementing a shape you have seen
twice is how you cement the wrong one. Wait until the repetition tells you what
the shape actually is.

**L3 when there is a customer-visible flow worth proving.** It is the most
valuable layer and the most expensive one; it earns its cost only once
something real can break.

---

## Stopping an agent from relaxing the rules

Every rule here is enforced by a file an agent can edit. The shortest path from
a red build to a green one is not fixing the code — it is disabling the rule,
widening a glob, or deleting a workflow step, and at the diff level all three
are indistinguishable from a fix. Two rules close that door.

**`L2.FACTORY_CONFIG_IS_LOCKED`** hash-locks the policy, the ratchet, the local
rules, the root allowlist, the CI workflow and the hooks. Editing any of them
without `sf lock` in the same commit fails. The lock does not make the edit
impossible — it makes it undeniable, as a second deliberate line in the diff on
a path a code owner watches.

**`L2.POLICY_ONLY_TIGHTENS`** reads the edit and decides which direction it
went:

```sh
sf check --changed origin/main
```

A rule disabled or removed, an exclusion added, a scope narrowed, a ceiling
raised, a gate weakened, a new violation frozen, a review date pushed out — all
fail. Tightening passes silently, so the rule never taxes the direction you
want.

Alongside them, `.allowed-root-files` blocks the `NOTES.md` / `PLAN.md` reflex,
the dependency lock turns adding a package into a reviewable act, and
`L2.NO_PERMANENT_EXCEPTION` fails the build when a frozen exception outlives its
review date.

---

## L6: hunting defect classes

`sf` does not reimplement a vulnerability database, a secret scanner or a race
detector. Those tools exist, they are better than anything that could live
here, and they differ per language. What is missing in most repositories is not
the tool — it is the guarantee that the tool is *still wired in*. So a rule
names a concern, and the check asserts that something covering it actually runs
in your CI or task runner.

| Concern | Python | TypeScript | Go | Rust | Ruby |
|---|---|---|---|---|---|
| Dependency vulnerabilities | pip-audit | npm audit, osv-scanner | govulncheck | cargo audit | bundler-audit |
| Committed secrets | gitleaks, detect-secrets, trufflehog (language-independent) | ← | ← | ← | ← |
| Insecure patterns | bandit, semgrep | semgrep, eslint-plugin-security | gosec | clippy, cargo-geiger | brakeman, semgrep |
| Dead code | vulture | knip, ts-prune | staticcheck | dead_code, cargo udeps | — |
| Data races | — | — | go test -race | ThreadSanitizer, loom | — |
| Performance regression | pytest-benchmark | vitest bench | go test -bench | criterion | — |

`sf init` writes these steps into the generated workflow for the languages you
selected. A concern with no listed tool for a language is not a violation —
that is a statement about the ecosystem, not about your repository.

### What static analysis cannot do

**Nothing decides whether a program deadlocks.** It is undecidable in general,
and a tool claiming otherwise teaches you to trust it wrongly. What *is*
decidable is the shape that causes the deadlocks and starvation people actually
ship, and two rules enforce exactly that:

- **`L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`** — a network call, a sleep, a
  subprocess or an `await` inside a region that holds a lock. Holding a lock
  across something slow turns mutual exclusion into a queue, which is
  starvation; awaiting while holding a synchronous lock can park the
  continuation on a thread that then blocks on that same lock, and nothing
  moves again.
- **`L6.ONE_LOCK_AT_A_TIME`** — a second lock acquired while the first is held.
  It cannot tell a correctly ordered pair from a dangerous one; ordering is a
  global property and the rule sees one function. It makes the second
  acquisition *visible*, which is the part that is otherwise invisible. If the
  pair is genuinely correct, freeze it in the ratchet with the ordering written
  beside it — now the ordering is documented, which is the only thing that ever
  prevents the inversion.

Data races get the same honesty: no static check finds them, so the rule
requires the dynamic detector to run instead.

---

## Layer 5 is the whole point

A check with a typo in its query, a glob that matches nothing, or a scope that
excludes the entire source tree passes silently forever — and reads exactly
like a check that works. A green build proves nothing about a rule that never
ran.

So every rule ships with the smallest repository that violates it, and
`sf verify` runs each rule against its own mutation:

```
$ sf verify

✓ L0.EXCEPTIONS_HAVE_ONE_HOME — 1 finding(s): `OrderRejectedError` is defined outside its allowed location
✓ L1.COMPLEXITY_CEILING — 1 finding(s): `price` has 6 independent paths, ceiling is 4
✓ L1.NO_BLANKET_SUPPRESSION — 1 finding(s): Bare `# noqa` disables every rule on the line...
✓ L4.DOC_LINKS_RESOLVE — 1 finding(s): link target `../src/pricing/README.md` does not exist
...
12/12 enabled rules proven to fire
```

The generated pre-commit hook and CI workflow run `sf verify` **before**
`sf check`, because a check that stopped firing is the cheaper failure to find
first.

---

## The gate: L3 in practice

`L3.GATE_HAS_FRESH_EVIDENCE` is the method's sharpest edge and the reason the
rest of it exists.

```yaml
# .software-factory/policy.yaml
gates:
  checkout:
    activation: ["src/checkout/**"]
    evidence: "evidence/checkout.json"
```

1. **Activation comes from touched paths.** Not a label, not a checkbox, not a
   sentence in a pull request. Touch `src/checkout/**` and the gate is on.
2. **The manifest is re-verified, never trusted.** `sf` re-reads the referenced
   report, recomputes its SHA-256, and re-checks every required assertion in
   the raw report. A summary cannot assert a pass the report never contained.
3. **An `unsupported` assertion is not a pass.** The most common way a green
   gate proves nothing is an assertion the harness could not evaluate being
   counted as one that succeeded.
4. **Evidence expires when the code moves.** The manifest records a digest of
   the activation paths. Change the implementation and the evidence dies with
   it, instead of quietly certifying something it never saw.
5. **The goal is checked for leaked answers.** A goal naming the source tree is
   a replay recipe, not a customer asking for something.

```sh
sf seal checkout   # recompute every digest from what is actually on disk
```

`seal` only recomputes digests — it cannot launder a failing report into a
passing one.

---

## What a rule looks like

The catalog is the portable asset. The binary is just what runs it.

```yaml
id: L0.EXCEPTIONS_HAVE_ONE_HOME
layer: L0
title: Error types live in one canonical module per domain
severity: high

statement: >-
  Define every error type in its domain's canonical errors module.
why: >-
  An agent asked to "add an error for X" will define it wherever it is already
  editing. Three months later nobody can answer "what can this domain fail
  with?" without reading every file.
fix: >-
  Move the class definition into the domain's errors module and import it back
  where it is raised.

check:
  kind: shape
  languages:
    python:
      query: |
        (class_definition
          name: (identifier) @name
          (#match? @name "(Error|Exception)$")) @target
    typescript:
      query: |
        (class_declaration
          name: (type_identifier) @name
          (#match? @name "(Error|Exception)$")) @target
    go:
      query: |
        (type_declaration
          (type_spec name: (type_identifier) @name
                     (#match? @name "(Error|Err)$"))) @target
defaults:
  must_live_in: ["**/exceptions.py", "**/errors.ts", "**/errors.go"]
```

`why` and `fix` are mandatory: the catalog refuses to load a rule missing
either. A rule with no reasoning is a wall an agent hits with no way to tell
whether it is protecting something or just old.

Structural rules are [tree-sitter](https://tree-sitter.github.io/) queries plus
a constraint on where matches may live. The engine knows nothing about
controllers, repositories or exceptions — that vocabulary lives entirely in the
catalog, which is what lets one rule mean the same thing in four languages.

A language may also carry an `unless` query: the same shape plus whatever makes
it acceptable, whose matches cancel the ones above on that line. It exists
because negation over siblings is not expressible in a tree-sitter query, and
some rules need it — `L1.SKIPPED_TESTS_STATE_A_REASON` asks a TypeScript skip
for a comment on the line above, because `it.skip('name', fn)` has no parameter
for a reason and a comment is the only place one can live.

```yaml
    typescript:
      query: |
        (expression_statement (call_expression ... )) @target
      unless: |
        ((comment) . (expression_statement (call_expression ... )) @target)
```

**Adding a language** is a grammar plus one query per rule you want it to
cover. **Adding a rule** is a YAML file in `.software-factory/rules/` and a
fixture under `.software-factory/mutations/<RULE_ID>/`.

**Adding a check** the structural kinds cannot express is `kind: command`: a
rule whose failure only a subprocess can decide (a schema export, a codegen
step, a linter this repo already trusts) reports a finding on nonzero exit,
with no fork required. It needs `sf check --allow-commands` to actually run —
see [Checks this tool cannot express](#checks-this-tool-cannot-express).

```yaml
check:
  kind: command
  run: "make export-openapi && git diff --exit-code -- contracts/"
```

Languages today: **Python, TypeScript/TSX, Go, Rust, Ruby.**

`sf verify` requires every language a rule declares to be shown tripping it,
otherwise three broken queries hide behind one that works.

---

## Rules that are only about one version of a dependency

A deprecation rule for Tailwind 3, or a shape rule for the QuickBooks v3 API,
is only correct while that version is the one installed. Give the instance a
`when`, and it activates from the manifest:

```yaml
rules:
  L1.NO_BLANKET_SUPPRESSION@tailwind3:
    enabled: true
    when:
      dependency: tailwindcss
      manifest: package.json
      version: "^3"
```

Once the pin moves to `^4`, that instance stops running, because it is about a
version this repository no longer has. It does not go quiet: `L5.NO_INERT_RULE`
names it, the range it was written for and the version found, so the answer is
to repoint it or remove it. A condition that silently disabled itself would
hand an agent a way to switch a rule off by editing a dependency.

A `when` naming a package no manifest declares is a finding for the same
reason, as is one naming a manifest that is missing or in a format this binary
cannot read. Every way of not deciding is reported; none of them is a skip.

- **The manifest range, never the lock.** The lock is more accurate and there
  are several lock formats per ecosystem. The range in the manifest is what
  the team decided, and the decision is what the rule is about. It is also
  already covered by `L2.DEPENDENCIES_CHANGE_DELIBERATELY`, so the input
  cannot move without a lock update in the same commit.
- **Manifests read:** `package.json`, `Cargo.toml`, `pyproject.toml`
  (PEP 621 and Poetry), `requirements*.txt`, `Gemfile`, `go.mod`.
- **Ranges accepted:** `^3`, `~1.2`, `>=5`, `<4`, and a bare series like `3`
  or `3.4`. The question is whether the pin is still in the series the rule
  was written for, which the release numbers answer on their own.

What this does not do is check that the rule's content is right for the
version it claims. A regex written for Tailwind 3 stays a regex written for
Tailwind 3 whether or not the condition matches; reading the upstream
changelog and turning it into patterns is a job for
[`factory-author`](#factory-author--when-you-hear-yourself-repeating-a-review-comment).

---

## Monorepos, and more than one repository

### One rule, different settings per package

A monorepo needs the same rule twice: a complexity ceiling of 12 in the new
packages and 20 in the one nobody has had time to split up. Write the policy
key as `RULE@name`:

```yaml
rules:
  L1.COMPLEXITY_CEILING:
    enabled: true
    options: {max: 12, scope: ["packages/api/**", "packages/web/**"]}

  L1.COMPLEXITY_CEILING@legacy:
    enabled: true
    options: {max: 20, scope: ["packages/legacy/**"]}
```

Both resolve to one catalog rule with one written reason, and each instance
gets its own findings and its own ratchet entries — so paying down the legacy
debt does not require touching the other packages' allowance.

### Repositories that are checked together

Some invariants only exist *between* repositories: a public contract and the
private service that serves it, a schema and the client generated from it.
Neither checkout can see the other, so nothing in either notices when they
drift.

Declare the other checkouts — usually symlinks to sibling clones — and one
policy governs all of them:

```yaml
project:
  name: acme-workspace
  languages: [python, typescript]
  roots: ["acme-public", "acme-private"]
```

Findings keep the declared prefix (`acme-private/packages/api/handler.py`), so
a rule reads the same wherever the checkout actually lives. Symlinks are
followed **only** for declared roots, never during the ordinary walk — a
package manager's symlink farm would otherwise be walked as source.

Each repository can still run its own `sf check` for its own rules. The
workspace runs the ones that are about the relationship.

### Checks this tool cannot express

Some drift is only decidable by regenerating: export the API schema, run the
generator, compare. No glob or query says that, and a hash lock cannot either,
because the artifact is *supposed* to change whenever its source does.

`L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE` runs a command instead:

```yaml
  L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE:
    enabled: true
    options:
      run: "make export-openapi && git diff --exit-code -- contracts/"
```

What that buys over a plain CI step is everything around it: the written reason
printed where it fails, a mutation fixture proving it still fails when it
should, a place in the same report, and a policy that cannot be quietly
loosened. It is also the bridge for a bespoke checker a team already has —
wrap it, and it gets the same treatment as everything else.

Commands are **refused unless asked for**:

```sh
sf check --allow-commands      # or SF_ALLOW_COMMANDS=1
```

A policy file is data that travels with a clone, so running whatever it says
would make cloning a repository dangerous. Without the flag the rule still
reports — a check that did not run is not a check that passed.

---

## Adopting rules a repository already breaks

```sh
sf ratchet --months 6
```

Every existing violation is frozen by a content-stable key. New ones fail
immediately. Each frozen set carries a `review_by` date, and
`L2.NO_PERMANENT_EXCEPTION` fails the build once it passes — the date does not
force the fix, it forces the conversation, on a day someone chose in advance
rather than never.

Keys are derived from content, not line numbers, so moving code around does not
silently un-freeze the ratchet — and adding a key by hand to silence a new
violation is a visible line in a reviewed diff.

Re-seeding recomputes the frozen keys, never the deadline: a date already
accepted survives, and a newly frozen violation does not reset the clock on the
debt beside it. This matters because `sf ratchet` is part of the prescribed
order after any guardrail change — a run that stamped today + `N` months on
every entry would push every deadline out on every unrelated change, which
`L2.POLICY_ONLY_TIGHTENS` then rejects. Renewing a date that has genuinely
expired is a deliberate edit to the file, with the reasoning in the pull
request, which is the conversation the date exists to force.

---

## Commands

| | |
|---|---|
| `sf init` | Scaffold policy, docs, CI, hooks, fixtures; seed locks and ratchet |
| `sf check` | Run every enabled rule. `--format json\|markdown`, `--changed <ref>`, `--rule <ID>`, `--allow-commands` |
| `sf verify` | Prove every enabled rule fires on its mutation fixture |
| `sf explain <RULE>` | What the rule requires, why it exists, how to fix a violation |
| `sf catalog` | List the rules. `--layer L0` |
| `sf interview` | The decision tree an interview walks. `--json` for an agent |
| `sf skills` | Install the agent skills. Asks where; `--project`, `--user` or `--dir` to say |
| `sf ratchet` | Freeze today's violations. `--months N` |
| `sf lock` | Rewrite hash locks from disk |
| `sf fixtures` | Write the mutation fixtures for every enabled rule |
| `sf docs` | Regenerate the rule sections of `docs/rules.md`, preserving your own prose |
| `sf seal <gate>` | Recompute the digests in a gate's evidence manifest |

Exit codes are hierarchical, so a caller can tell "the tool could not run" from
"the repository has violations": `3` bootstrap failed, `2` config error, `1`
findings, `0` clean.

---

## The interview

Bare `sf init` cannot know whether you use repositories or call the ORM from
services, whether errors live per-module or in one file, or which package the
client must never import. So it enables the generic rules and leaves the
structural ones off.

The interview fixes that, and it is deliberately split in two:

- **The conversation belongs to the agent.** It reads your repo, answers what
  the code can answer, and pushes back when you say "layered" and a route
  handler opens a database connection.
- **The mapping does not.** Which rules an answer produces lives in
  `sf interview`, as data. Two agents interviewing the same team land on the
  same policy — otherwise it is just each agent's taste with extra steps.

```sh
sf interview          # the decision tree, and what each answer enforces
sf interview --json   # the same, for an agent conducting it
```

Twelve decisions, walked as a tree in rounds: what the repository is, how it is
organised, which framework, how code reaches the database, where error types
live, what validates the boundary and where those schemas sit, how the client
fetches and stores state, which packages the client may never import, whether
anything shares mutable state across threads, and what is generated rather than
written.

Answers go in a file, and the file is the decision:

```yaml
# .software-factory/answers.yaml
version: 1
answers:
  kind: backend-service
  architecture: hexagonal
  framework: fastapi
  data_access: repositories
  errors_home: per-module
  validation: pydantic
  validation_placement: with-the-handler
  concurrency: shared-state
  generated: "src/generated/**, **/*_pb2.py"
```

```sh
sf init --language python --layer L1,L4,L5,L6 --answers .software-factory/answers.yaml
```

That enables the L0 rules those answers justify (and only those), points them
at the right directories, switches off the ones that cannot mean anything here,
instantiates **repo-specific rules from templates** with your own package names
filled in — each with a fixture proving it fires — and writes
`docs/architecture-decisions.md` recording who decided what.

A real example, on a TypeScript monorepo with a Next.js client:

```
$ sf init --language typescript --layer L1,L4,L5,L6 --answers answers.yaml
  .software-factory/rules/client-never-imports-the-data-layer.yaml
  .software-factory/rules/no-fetch-inside-an-effect.yaml
  docs/architecture-decisions.md
  .software-factory/ratchet.yaml (118 existing violations frozen)

$ sf verify
14/14 enabled rules proven to fire

$ sf check
✓ 14 rules, no findings (118 frozen by the ratchet)
```

The generated `L0.CLIENT_NEVER_IMPORTS_THE_DATA_LAYER` carries that repo's
actual package names in its tree-sitter query and its own `apps/web/**` in the
constraint. It found zero violations — the boundary already held, and now
nothing can quietly break it.

Change an answer and re-run. Do not hand-edit the generated policy, or the
decision record stops describing what is enforced.

---

## The skills

Four agent skills for Claude Code, shipped inside the binary so they cannot
drift out of step with the `sf` they drive. Install them once:

```sh
sf skills             # asks: this repository, or every project
sf skills --project   # <root>/.claude/skills, no question
sf skills --user      # ~/.claude/skills, no question
```

**Invoke them by name.** `/factory-init`, `/factory-triage` and so on: an agent
may pick one up from its description when the conversation matches, but that is
not something to rely on, and a skill that silently did not load looks exactly
like one that did and had nothing to say.

Their job is to **author policy and produce evidence**, never to remember rules;
that is what the binary is for.

One boundary runs through all four: **an agent proposes policy, a human merges
it.** That is the only thread separating a factory from a system grading its own
homework, and no amount of tooling substitutes for it.

### `factory-init` — setting up, or when the architecture changes

The one you use first. It runs the interview above.

> **You:** `/factory-init set up software-factory in this repo`

It reads the codebase before asking anything, answers what the code can answer,
and asks in rounds — each question numbered, each with a recommendation:

> ❓ **Q2** — **Architecture**: I can see `packages/*/domain/`,
> `application/` and `infrastructure/`, so this looks domain-driven. But
> `apps/api/src/routes/users.ts` opens a database connection directly. Is the
> layering the intent or the reality?
>
> ➡️ I'd answer `ddd` and freeze today's 40 violations with a six-month review
> date, rather than `none-yet` — but I want you to pick that deliberately,
> because it is a commitment to fix them.

Then it applies the answers, and reads the result back to you in numbers: what
was frozen and when it comes due, which rules were switched **off** and why,
which repo-specific rules were generated. Re-run it whenever an architectural
decision changes.

### `factory-author` — when you hear yourself repeating a review comment

You are lead on a TypeScript monorepo. It is the third time this month you have
written *"don't import the db directly in a component, go through the API"*.

> **You:** `/factory-author` third PR this month where someone imports
> `@acme/db` inside `apps/web`. I want this to stop being my comment and
> become a check.

It does not say "good idea, I'll remember". It checks the rule does not already
exist (`sf catalog`), writes the YAML with a mandatory `why` — written for the
person who will want to delete this rule in a year — writes the smallest
repository that violates it, runs `sf verify --rule`, and **if it does not fire
it fixes the rule, not the fixture.** Then `sf ratchet` if there is existing
debt, and it tells you how many it froze and when they come due.

What you get is a pull request: a rule, a fixture, a section in the rules
document. You read it and merge it.

Its other trigger is opening a phase of work — *"I'm rewriting billing, I want a
completion gate"* — where it designs the activation paths and required
assertions, and insists the assertions be observations read back from the
system, not claims the actor makes about itself.

### `factory-evidence` — when a gate is red and you have to prove something

> **You:** `/factory-evidence` the checkout gate is `stale` and I need to
> merge today.

`stale` means the code changed since the evidence was sealed. The tempting move
— and the one the skill is explicitly told not to make — is to re-run `sf seal`
and move on. That is a lie with a hash on it.

It runs the real thing: starts the app, drives it through the entry point a
customer would use, collects the observations, writes the report, and only then
seals. And it tells you plainly when an assertion did not pass:

> *"`refund.settled` came back `unsupported` — the harness could not evaluate
> it. That is not a pass. The finding is the product's behaviour, not the gate."*

If the gate cannot go green, it stops and says so. A gate that will not pass is
usually reporting a real defect, and the defect is worth more than the build.

### `factory-triage` — the daily one

> **You:** `/factory-triage` CI is red, 12 findings, sort it out.

It reads the report (which already carries each rule's reasoning) and works in a
specific order: **`sf verify` failures first** — a rule that stopped firing
means every green build since then proved nothing.

The part that earns its disk space is the list of what is **not** a resolution:
widening a glob, adding a ratchet key for a violation you just wrote, pushing a
`review_by` out, disabling the rule, suppressing at the source. In a diff, each
of those is indistinguishable from a fix. The skill is told to name which one it
would pick, say why, **and stop**.

It has four honest resolutions, and one of them is *fix the rule*. If the rule
is wrong, you change it deliberately, update its `why`, and re-run `verify`.
That happened while building this: the `Exit condition` marker did not accept
`**bold**`. The rule was wrong, not the document.

The safety net underneath is mechanical now — if it (or any agent) tries to
disable a rule to go green, `L2.POLICY_ONLY_TIGHTENS` catches it, and it
survives even if the agent runs `sf lock` to cover the trail.

---

## What this is not

It does not replace your linter, type checker or test suite — it enforces the
decisions those tools have no opinion about. It does not review code for
correctness. It does not run your tests; L3 checks that *something* ran, proved
what it claimed, and has not gone stale since.

And it is deliberately small. A method you cannot read in an afternoon is a
method nobody will adopt.

---

## This repository checks itself

`sf` is written in Rust and Rust is one of its target languages, so this
repository runs its own rules against its own source, with its own mutation
fixtures, in its own CI: **30 rules enabled, 30 proven to fire, no findings.**
Four of the 34 are switched off here and one is frozen with a review date,
each as a written decision in [`docs/rules.md`](docs/rules.md) — because
`L5.NO_INERT_RULE` refuses to let a rule be enabled and pointed at nothing.

See [`docs/method.md`](docs/method.md) for the reasoning behind the layering.

## Contributors

[![Contributors](https://contrib.rocks/image?repo=nicolasmelo1/software-factory)](https://github.com/nicolasmelo1/software-factory/graphs/contributors)

## License

MIT.
