# `L1.SKIPPED_TESTS_STATE_A_REASON` in every language `sf` parses

The rule shipped one query, for Python. `sf` parses four languages, so in the
other three the rule was enabled-and-inert — which is the state
`L5.NO_INERT_RULE` exists to make impossible. Both repositories that use this
tool had already written the workaround down:

- this one, in `.software-factory/policy.yaml`: *"Disabled: the rule ships a
  Python query only, and this is a Rust repository."*
- `nicolasmelo-portfolio`, in `docs/rules.md`: *"The catalog ships a
  python-only tree-sitter query for this rule and this repository is
  typescript-only, so enabling it could never produce a finding. Turn it back
  on if a python query lands upstream."*

Two repositories disabling the same rule for the same reason is not a policy
decision, it is a missing query. A skipped test that reports green is not a
Python problem.

## Why the three new queries are not one query

Go and Rust are the direct translation of the Python one — the reason is an
argument the API accepts and the violation is the call made without it:
`t.Skip()` against `t.Skip("...")`, `#[ignore]` against
`#[ignore = "..."]`. Both are matched with anchors so the *presence* of the
reason is what cancels the finding, which keeps the rule's meaning identical
across three languages.

TypeScript is the one that needed a decision. `it.skip('name', fn)` has no
parameter for a reason, and neither jest nor vitest ever added one, so the
direct translation is a query that can never fire — the same inert rule, now
with a query to make it look covered. Two other options were considered and
rejected:

- **Flag every skip, and freeze the legitimate ones in the ratchet.** This is
  what `L6.ONE_LOCK_AT_A_TIME` does with a lock pair it cannot judge, and the
  ratchet is a better place for a reason than a comment: it carries a
  `review_by` date and `L2.NO_PERMANENT_EXCEPTION` fails when it expires. It
  was rejected because it makes the rule mean something different in
  TypeScript than the title claims — "a skipped test says why" becomes "there
  are no skipped tests" — and rule ids are a public contract.
- **A line pattern.** Already argued against in the rule's own `why`: a real
  skip spans several lines and a line-based check flags the correct ones,
  which teaches people to ignore the rule.

What is left is the comment on the line above, which is the only place
JavaScript leaves for a reason. It is weaker evidence than a `reason=`
argument in exactly one way — nothing parses it — and identical in every other
way, including that both are prose nobody verifies.

## What the engine was missing

"This shape, unless a comment sits above it" is not expressible as one
tree-sitter query: negation over siblings does not exist in the query
language. `LangQuery` therefore gained an optional second query, `unless`,
matching the same shape *plus* what makes it acceptable; `shape.rs` subtracts
its `@target` lines from the first query's. Two positive queries and a set
difference, which is the same shape as `forbidden`/`unless` in the
`text_pattern` rules — the reader already knows the idiom.

Deliberately out of scope: the three concurrency rules that
`nicolasmelo-portfolio` also documents as not enabled
(`L6.DATA_RACES_ARE_DETECTED`, `L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`,
`L6.ONE_LOCK_AT_A_TIME`). Those are not missing queries. JavaScript has no
lock to hold and no shared mutable state across threads to race on, and the
ecosystem ships no dynamic race detector to wire in. Writing a TypeScript
query for them is exactly the "inventing a query so the coverage table looks
full" that [expand-language-adapters.md](expand-language-adapters.md) warns
against.

**Exit condition:** a TypeScript repository with a bare `it.skip` fails
`sf check` on it, this repository enables the rule on its own Rust source
instead of documenting why it cannot, and `sf verify` proves the rule fires in
all four languages.

## Acceptance criteria

- [x] The rule fires on a bare skip in python, typescript, go and rust, and
      `sf verify` refuses to pass on three of four
      (proof: test:.software-factory/mutations/L1.SKIPPED_TESTS_STATE_A_REASON/)
- [x] A TypeScript skip with a comment on the line above produces no finding,
      while the same skip without one does
      (proof: test:src/checks/shape.rs::cancelling_query)
- [x] This repository enables `L1.SKIPPED_TESTS_STATE_A_REASON` and stays green
      (proof: test:.software-factory/policy.yaml)
- [ ] `nicolasmelo-portfolio` re-enables the rule and deletes its
      "deliberately disabled" section
      (proof: deferred:a pull request in that repository, once this is merged
      and its CI installs an `sf` that carries the query)
