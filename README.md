# software factory

**A method for building software with agents, packaged as a single binary that runs against any repository.**

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

Everything else in this repository is a consequence of that sentence.

---

## Quick start

```sh
cargo install --git https://github.com/nicolasmelo1/software-factory --locked

cd ~/code/your-project
sf init                    # policy, docs, CI, hooks, mutation fixtures, seeded ratchet
git config core.hooksPath .githooks

sf verify                  # prove every enabled check actually fires
sf check                   # see what is live
```

`sf init` is not a config file drop. It writes the enforcement *and* the
fixtures that prove the enforcement works, generates the document that explains
each rule, and freezes today's violations so the rules can be adopted by a
repository that already breaks them — new violations fail from the first run.

```
$ sf check

✗ high  L0.PERSISTENCE_STAYS_IN_REPOSITORIES — Data-access calls stay in the persistence layer
  why  This is the boundary agents erode fastest, because reaching for the
       session directly is always the shortest diff. Once a controller runs
       its own query, transaction scope, N+1 behaviour and test seams stop
       being properties of one layer and become properties of the whole
       codebase.
  fix  Move the query into a repository method that names the intent
       (`find_active_orders_for`, not `execute`) and call that instead.
    src/orders/controllers/refund.py:6 — `execute` is defined outside its allowed location
       expected **/repositories/**, **/repository/**, **/dal/**
       actual   src/orders/controllers/refund.py
```

That output shape is deliberate. The failure message is the only documentation
an agent reliably reads, so every rule carries its reasoning to the point of
failure instead of leaving it in a document nobody opens.

---

## The six layers

| | Layer | What it checks | Cost to adopt |
|---|---|---|---|
| **L0** | Shape | Where things live — error types, data access, entrypoints, layer boundaries | Needs a known shape |
| **L1** | Grain | How code reads — complexity ceiling, banned escape hatches, no blanket suppressions | One hour |
| **L2** | Contract | No drift between a source of truth and what derives from it — hash locks, dated exceptions | Needs a second surface |
| **L3** | Effect | A real actor achieved the observable outcome, and the evidence has not gone stale | Needs a customer-visible flow |
| **L4** | Cadence | Docs, plans and rules stay attached to each other | Three markdown files |
| **L5** | Meta | Every check is proven to fire | Free, and everything else depends on it |

Run `sf catalog` for the rules, `sf explain <RULE>` for the reasoning behind any
one of them.

### Adopt in this order

**Day one: L1, L4, L5.** This is `sf init`'s default, and it is a deliberate
recommendation rather than a shortcut. L1 costs an hour. L4 costs three
markdown files. L5 is what makes either of them mean anything.

**L0 after the third occurrence of a pattern.** Cementing a shape you have seen
twice is how you cement the wrong one. Wait until the repetition tells you what
the shape actually is.

**L2 when a second surface derives from a first** — a generated client, a
schema and its migrations, a design token and its stylesheet.

**L3 when there is a customer-visible flow worth proving.** It is the most
valuable layer and the most expensive one; it earns its cost only once
something real can break.

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

**Adding a language** is a grammar plus one query per rule you want it to
cover. **Adding a rule** is a YAML file in `.software-factory/rules/` and a
fixture under `.software-factory/mutations/<RULE_ID>/`.

Languages today: **Python, TypeScript/TSX, Go, Rust.**

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

---

## Commands

| | |
|---|---|
| `sf init` | Scaffold policy, docs, CI, hooks, fixtures; seed locks and ratchet |
| `sf check` | Run every enabled rule. `--format json\|markdown`, `--changed <ref>`, `--rule <ID>` |
| `sf verify` | Prove every enabled rule fires on its mutation fixture |
| `sf explain <RULE>` | What the rule requires, why it exists, how to fix a violation |
| `sf catalog` | List the rules. `--layer L0` |
| `sf ratchet` | Freeze today's violations. `--months N` |
| `sf lock` | Rewrite hash locks from disk |
| `sf seal <gate>` | Recompute the digests in a gate's evidence manifest |

Exit codes are hierarchical, so a caller can tell "the tool could not run" from
"the repository has violations": `3` bootstrap failed, `2` config error, `1`
findings, `0` clean.

---

## Skills

`skills/` holds three agent skills. Their job is to *author policy and produce
evidence* — never to remember rules, which is what the binary is for.

| Skill | Role |
|---|---|
| `factory-author` | Turn a requirement into policy: gates, activation paths, required assertions |
| `factory-evidence` | Run the proof and seal a manifest that survives re-verification |
| `factory-triage` | Read a report, explain what actually broke, fix it or argue the rule |

The rule they all enforce: **an agent proposes policy, a human merges it.** That
is the only thread separating a factory from an agent grading its own work.

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
fixtures, in its own CI. See [`docs/rules.md`](docs/rules.md) for what is
enabled here and [`docs/method.md`](docs/method.md) for the reasoning behind
the layering.

## License

MIT.
