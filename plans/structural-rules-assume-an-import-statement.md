# The four L0 structural rules assume an import statement

Measured today, upgrading PostPilot (Rails 7.2.3.2, ~4,700 `.rb` files) from an
L4/L5 `sf` install to a Ruby-aware one, with `languages: [ruby]` and every L0
rule enabled:

- `L0.NO_CROSS_LAYER_IMPORT` — 0 findings.
- `L0.ONE_ENTRYPOINT_PER_FILE` — 0 findings.
- `L0.PERSISTENCE_STAYS_IN_REPOSITORIES` — 0 findings.
- `L0.EXCEPTIONS_HAVE_ONE_HOME` — 68 findings, the only one of the four that
  fires at all.

None of that means PostPilot's layering is clean. It means three of the four
rules query for something Rails does not do, and the fourth queries for
something Rails does differently than the model assumes.

`L0.NO_CROSS_LAYER_IMPORT`'s Ruby query (added by
[the Ruby language adapter](ruby-language-adapter.md)) matches
`require`/`require_relative` calls whose string argument names an internal or
generated path. That query was written honestly about its own limit: Rails
autoloads `app/` with no import statement at all, so the query has nothing to
match there by design, and it fires correctly on the `lib/`, gem and
`config.ru`-level requires that remain — PostPilot's explicit requires simply
never happen to hit an internal/generated path, hence zero.
`L0.ONE_ENTRYPOINT_PER_FILE` and `L0.PERSISTENCE_STAYS_IN_REPOSITORIES` ship no
Ruby query at all, for reasons the same plan already argued explicitly: a
route table centralised in `config/routes.rb` is the Rails idiom, not a
violation of "one entrypoint per file", and idiomatic ActiveRecord has models
query themselves with no separate repository layer to check placement
against. `L0.EXCEPTIONS_HAVE_ONE_HOME` is the one that fires, using the same
`Error`-suffix convention as the Python and TypeScript queries — but its
`must_live_in` model is "one `errors.rb` per package", and idiomatic Ruby
instead nests the error class inside the class that raises it
(`class OrderService; class NotEligibleError < StandardError; end; end`), a
shape no path glob can express. Most of the 68 are that idiom, not a real
violation.

## What graphify found instead

A `graphify` knowledge graph of PostPilot (27,031 nodes, 32,101 edges, built
from commit `627b256c`) finds violations none of the four rules above can see,
because they are constant references, not `require` statements:

- **Reverse dependencies into `app/controllers`: 9.** 1 from `app/models`
  (`shopify_store.rb:112`), 5 from `app/services` (two Expandfi webhook
  services, a deployment backfill, and
  `campaign_strategies/csv_export_service.rb:20` reaching into a
  controller-owned concern), 3 from `app/components`.
- **Controllers reaching the database directly.**
  `app/controllers -> app/models`: 343 edges.
  `app/controllers -> app/services`: 147 edges. 123 of 357 controller files
  (34%) call a model directly, and even 10 `.erb` views call `.find_by`
  inline.
- **A bypassed query layer.** `app/finders` (27 files) exists specifically to
  hold query logic and gets 16 inbound edges from services and 10 from
  controllers, against 595+343 direct model accesses from those same two
  layers.
- **A genuine positive.** `graphify path ApplicationConsumer
  ApplicationController` finds no path in either direction — the Karafka
  consumer boundary is real, and worth keeping that way.

A rule reading constant references instead of `require` statements would have
9 findings on day one, plus a defensible case for widening enforcement to the
controller/model boundary afterward.

## The design question

Rails autoloading means there is no import to match — `FooController` is
available in `app/services/bar_service.rb` with no `require`, no `use`,
nothing today's `shape` vocabulary sees, because Rails resolves the name at
load time from a path convention, not from a statement in the file. The only
signal left is the constant reference itself, read against the path
convention Zeitwerk already enforces: `app/services/foo.rb` defines `Foo`, and
a file under `app/services/**` referencing a constant matching `Controller$`
is a reverse edge.

This does not need a new cross-file symbol resolver. `shape`'s existing
query-plus-placement model already works this way in one direction:
`L0.EXCEPTIONS_HAVE_ONE_HOME` matches a name suffix (`Error$`) and constrains
where the *defining* node may live. A Rails cross-layer rule matches the same
kind of name suffix (`Controller$`) but constrains where the *referencing*
node may live (`must_not_live_in: app/services/**, app/models/**,
app/components/**`, or symmetrically `must_live_in: app/controllers/**`). No
new check kind — a new query, and placement read off the matched node's own
file the same way it already is, just measured against a different rule than
"where was this defined".

Named honestly, not glossed over:

- **A constant reference is not always a dependency.** `FooController.new` and
  `x.is_a?(FooController)` are both dependencies; `FooController` inside a
  comment or a string literal is not. The same treatment
  `L0.NO_CROSS_LAYER_IMPORT`'s query already gives string arguments would need
  to apply here too — matching `(constant)` nodes, not `(string)` nodes.
- **`const_get("FooController")` and metaprogrammed dispatch are invisible.**
  A tree-sitter query sees syntax, not runtime string construction. This is a
  real gap, not a rounding error — PostPilot's webhook services lean on this
  pattern — and it means any finding count from this rule is a floor, not a
  ceiling.
- **Rails' own base classes are fan-in, not a violation.** Every controller in
  every Rails app references `ApplicationController` by inheriting from it,
  and `ApplicationRecord`/`ApplicationJob`/`ApplicationMailer` are the same
  shape for their own layers. A naive `Controller$` query fires on every one
  of them, from everywhere. `kind: shape` already has the mechanism for
  exactly this: `unless`, the cancelling query
  `L1.NO_BLANKET_SUPPRESSION` uses for `# noqa: CODE` beside a bare
  `# noqa`. The exclusion here would name the Rails convention
  (`Application(Controller|Record|Job|Mailer)`), not a per-project value,
  because it is the framework's naming, not PostPilot's.

## Where this rule would live

Three shapes. This plan does not decide among them — that decision belongs to
the maintainer, made with the numbers above in hand, rather than folded into a
bug-fix pull request:

1. **Widen `L0.NO_CROSS_LAYER_IMPORT`'s existing Ruby query.** Cheapest to
   ship: the rule already exists, is already enabled, and already carries the
   "private/internal/generated" vocabulary. But the statement changes from "a
   require crossing a marked boundary" to "a reference crossing an
   architectural layer" — a different rule wearing the same id, with
   `must_live_in` defaults (`**/_internal/**`, `**/generated/**`) that mean
   nothing for `app/controllers/**`. Reusing the id without reusing the
   meaning is the same failure [the Ruby language
   adapter](ruby-language-adapter.md) took care to avoid when it refused to
   port `L0.PERSISTENCE_STAYS_IN_REPOSITORIES` literally.
2. **A new Rails-specific L0 rule.** Its own id, its own `why`, its own
   `must_not_live_in` defaults naming `app/controllers/**` as the protected
   layer. Honest about being a different rule, and the only option that can
   carry a Rails-specific `why` — "controllers are the interface layer;
   nothing below them reaches back up" — instead of stretching a
   language-neutral one to fit. The cost: a rule whose query exists only for
   Ruby, in a catalog that otherwise keeps every rule portable across its five
   languages, even when a language deliberately refuses a query. This one
   would refuse by construction for every language but Ruby.
3. **A `templates/` parameterised rule, instantiated by the interview.**
   [`templates/client-never-imports-the-data-layer.yaml`](../templates/client-never-imports-the-data-layer.yaml)
   is the closest existing shape: a `kind: shape` rule with `@@…@@` holes
   filled in per project instead of hard-coded. The Rails form would
   parameterise the protected layer's directory and constant suffix —
   `app/controllers/**` and `Controller$` for the common case — so a project
   with a differently named interface layer answers differently at `sf init`
   time. This is also the shape that admits PostPilot's `app/finders` bypass
   (16 inbound edges from services and controllers against 595+343 direct
   model accesses) is a second, separately answerable instance of the same
   template, rather than a reason to make the first rule bigger.

Whichever shape is chosen, the query needs one thing the existing four rules
never did: read the referencing file's own path to decide whether to fire, not
only the matched node's path. `shape`'s `must_not_live_in` already checks the
location of the *matched* node; a Rails cross-layer rule constrains that same
matched-node location, so no new placement primitive is required — only a
query whose `@target` is the constant reference itself, which already lives
inside the file being checked against `must_not_live_in`, the same way
`L0.EXCEPTIONS_HAVE_ONE_HOME`'s `@target` is the class definition it checks
against `must_live_in`.

## The `L0.EXCEPTIONS_HAVE_ONE_HOME` sub-problem

68 findings, all against `must_live_in: **/errors.rb`. Two honest options:

1. **A Ruby-specific placement predicate meaning "nested inside the class that
   raises it".** `must_live_in`/`must_not_live_in` are sets of path globs;
   "nested in the raiser" is a structural relationship between two AST nodes,
   not a location. This needs new `shape` vocabulary — something like
   `must_be_nested_in: <a query matching the enclosing class>` — not a new
   value for an existing option, and it would be the first placement
   predicate in the catalog that is not a glob.
2. **Accept nesting as a legitimate home, and say so in the rule's Ruby form**
   — the way [the Ruby language adapter](ruby-language-adapter.md) already
   accepted no-Ruby-form at all for two other rules rather than force a Rails
   shape onto them. This is not "widen the glob to make the finding
   disappear": the finding disappears because the *model* — one canonical
   file per domain's errors — does not describe the idiom the way it
   describes a Python or TypeScript exceptions module, and that argument is
   made here, in this document, instead of silently inside an unrelated diff.

The two are not mutually exclusive. An agent could ship option 2 immediately —
the query already fires; only its Ruby placement predicate needs restating, or
dropping — and treat option 1 as a separate, larger `shape` capability with
its own plan, useful beyond this one rule.

**Exit condition:** `sf check --changed` against a Rails repository with
`languages: [ruby]` reports a reverse-dependency finding for at least one of
the 9 call sites graphify found in PostPilot (`shopify_store.rb:112`, either
Expandfi webhook service, the deployment backfill,
`campaign_strategies/csv_export_service.rb:20`, or any of the 3
`app/components` cases), reports zero findings on
`ApplicationController`/`ApplicationRecord`/`ApplicationJob`/`ApplicationMailer`
inheritance anywhere in that same repository, and `L0.EXCEPTIONS_HAVE_ONE_HOME`'s
Ruby form either fires only on genuinely misplaced exceptions or states in
`docs/rules.md` why nesting is accepted — not the 68-finding, undocumented
state measured today.

## Acceptance criteria

- [ ] The design decision — which of the three shapes above, and which id —
      is made and recorded in `docs/rules.md` before any query ships
      (proof: unspecified:this is a decision for the maintainer to make, not
      something a check can validate)
- [ ] The chosen shape's query fires on `shopify_store.rb:112` or an
      equivalent fixture reproducing the same reverse-dependency shape, and
      does not fire on any `ApplicationController`/`ApplicationRecord`/
      `ApplicationJob`/`ApplicationMailer` reference
      (proof: deferred:no query is written yet)
- [ ] `sf verify` proves the new or widened rule fires on its own mutation
      fixture in `src/fixtures.rs`, per `L5.EVERY_CHECK_HAS_A_MUTATION_TEST`
      (proof: deferred:no fixture is written yet)
- [ ] Pointed at PostPilot with `languages: [ruby]`, `sf check` reports at
      least one of the 9 measured reverse-dependency findings and zero
      base-class false positives
      (proof: deferred:blocked on the query above)
- [ ] `L0.EXCEPTIONS_HAVE_ONE_HOME`'s Ruby `must_live_in` either gains a
      nesting-aware placement predicate or its `docs/rules.md` entry states
      plainly that nesting inside the raiser is an accepted home, so the rule
      stops reporting the current 68 mostly-idiomatic findings as violations
      (proof: deferred:depends on which of the two sub-problem options is
      chosen)
- [ ] No existing rule's `must_live_in`/`must_not_live_in` for another
      language is touched by this work
      (proof: unspecified:an absence, enforced by review of the diff that
      implements this plan)
