# Ruby language adapter

Executes [expand the language adapters](expand-language-adapters.md), which
already named Ruby as the obvious next candidate. `Lang::Ruby` is wired
through every arm of `src/lang.rs`: `name`, `grammar` (`tree-sitter-ruby`
0.23.1, which builds cleanly against this repository's `tree-sitter` 0.26.12
— proven by `cargo build --release`, not assumed from the version numbers),
`from_path` (`.rb`, `.rake`, `.gemspec`, `.ru`), `from_name`,
`function_kinds` (`method`, `singleton_method`), `branch_kinds` and
`boolean_operator_kinds`/`boolean_operators` (`&&`, `||`, and Ruby's
low-precedence keyword forms `and`/`or`, which tree-sitter-ruby's grammar
folds into the same `binary` node and the same `operator` field as the
symbolic operators).

## A real bug the grammar exposed, not invented

`tree-sitter-ruby` names quite a few of its statement-level nodes after their
own opening keyword: the named `if` node, the named `elsif` node, `unless`,
`while`, `until`, `when`, `rescue`, `for`, `case`, `begin` and `ensure` are
*all* also present, as a completely separate anonymous token, as a child of
that very node — the literal keyword that opens it. `Node::kind()` returns
the same string for both. `src/checks/complexity.rs`'s walk pushed *every*
child onto its stack, named and anonymous alike, so a bare `if x > 0 ... end`
scored 2 instead of 1: once for the real `if` node, once again for its own
embedded `if` keyword token. A method with eleven `return … if …` modifiers
(`app/components/gen/badge_component.rb#badge_colors` in PostPilot) scored 23
independent paths instead of 12 — not a missing query silently matching
nothing, but a query that matched twice, silently, for every real branch.
None of the other four languages' grammars name a rule identically to one of
their own keyword tokens, so this never showed up before Ruby.

The fix is in `src/checks/complexity.rs`, not in `src/lang.rs`: the walk now
uses `Node::named_children()` instead of `Node::children()`, in both the
function-discovery loop and the branch-counting loop. This is
behaviour-preserving for Python, TypeScript, Go and Rust — verified by
`sf verify` reporting the same finding counts for their fixtures before and
after — and it is what makes it safe to give Ruby its full, real branch
vocabulary (`if`, `elsif`, `unless`, `while`, `until`, `for`, `when`,
`in_clause`, `rescue`, `conditional`, plus the five modifier forms) instead
of quietly dropping every block-form conditional to dodge the bug.

## Queries added, and why

Required by the gates: **`L1.SKIPPED_TESTS_STATE_A_REASON`** — RSpec's bare
`skip`/`pending` (no reason) and `xit` (which structurally cannot carry a
reason — it only ever takes the example description), plus Minitest's bare
`skip`. `skip("a real reason")` is not flagged; the query distinguishes a
bare `(identifier)` statement from a `(call … arguments: (argument_list))`
with content, which is exactly the same shape the Python query already
draws.

Judged on evidence and included:

- **`L0.EXCEPTIONS_HAVE_ONE_HOME`** — Ruby's own idiom for a domain error is a
  `StandardError` subclass named with the `Error` suffix
  (`OrderRejectedError < StandardError`), the same suffix convention the
  Python and TypeScript queries already use. `must_live_in` gained
  `**/errors.rb`.
- **`L0.NO_CROSS_LAYER_IMPORT`** — Rails autoloads `app/` with no import
  statement at all, so this rule has nothing to say about the majority of a
  Rails codebase. But `require`/`require_relative` are real, explicit
  imports everywhere a Ruby codebase actually uses them: `lib/`, gems,
  engines, rake tasks, scripts, `config.ru`. The query matches those, the
  same way the TypeScript query matches `import` and the Rust query matches
  `use`.
- **`L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`** and **`L6.ONE_LOCK_AT_A_TIME`**
  — Ruby's `Mutex`/`Monitor` idiom is `lock.synchronize { … }` (or
  `lock.synchronize do … end`), plus the explicit `.lock`/`.try_lock` pair.
  Both queries key off a receiver whose name reads as a lock
  (`(?i)(lock|mutex|semaphore)`), the same heuristic the Python `with`-based
  query already uses for the same rules.

## Two deliberate refusals

**`L0.ONE_ENTRYPOINT_PER_FILE`** ships no Ruby query. Rails centralises
routes in `config/routes.rb` by convention — one file declaring the whole
route table is the idiom, not a violation of it — so "a file that declares a
route declares exactly one" has no honest Ruby form. Inventing one would
either fire on every Rails app's `routes.rb` (which is correct Rails, not a
finding) or require guessing at a controller-action convention this rule was
never written to check.

**`L0.PERSISTENCE_STAYS_IN_REPOSITORIES`** ships no Ruby query either.
Idiomatic ActiveRecord has no `db`/`session` receiver to match against —
`Order.where(...)`, `order.update!(...)` — models query themselves. Porting
this rule literally would mean either matching on ActiveRecord's own class
methods (which is the pattern this rule exists to *allow* elsewhere, since
"the persistence layer" already *is* the model in Rails) or inventing a
"repository" convention PostPilot does not use. Both are a different rule
wearing this one's id, which is exactly the failure mode
`expand-language-adapters.md` warns about. If a future change adopts an
explicit repository layer on top of ActiveRecord, this is the rule to widen
— deliberately, with its own reasoning, not as a side effect of adding a
language.

## Verification

`sf verify` proves all five queries fire, in Ruby specifically — not merely
leaving the rule green overall — via `src/fixtures.rs`'s ruby fixtures.
`L1.SKIPPED_TESTS_STATE_A_REASON` is disabled in this repository's own
policy (`sf` is a Rust project; the comment on that line already said so
before this change and remains true), so its fixture is proven the same way
every other rule this repository doesn't apply to is proven: independently,
via `sf init --language ruby --layer L1` in a scratch repository, where it is
enabled and `sf verify` shows it firing.

Pointed at a mirror of `~/Sites/pp-team/postpilot`'s Ruby source (read-only;
never modified), `L1.COMPLEXITY_CEILING` — which reported zero findings
against that repository's 365,133 lines of Ruby before this change, because
`Lang::from_path` could not classify `.rb` at all — now reports real ones,
for example `app/services/campaign_strategies/total_calculator.rb:10`,
`calculate`, 34 independent paths against a ceiling of 12: a 19-way
`case`/`when` dispatch with embedded ternaries, an `if`/`else`, and a
`rescue`, which any Ruby maintainer would recognise as a real candidate for
extraction into one formatter per column instead of one branch per column.

**Exit condition:** a Ruby repository runs `sf init`, `sf verify` reports
every enabled rule firing on its fixture including every Ruby query, and
`sf check` produces a finding a Ruby maintainer agrees is real — met by the
verification above.
