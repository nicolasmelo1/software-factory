# What a rule looks like

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
see [Checks this tool cannot express](policy.md#checks-this-tool-cannot-express).

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
[`factory-author`](../start/the-skills.md#factory-author--when-you-hear-yourself-repeating-a-review-comment).
