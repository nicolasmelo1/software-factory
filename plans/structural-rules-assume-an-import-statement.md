---
sources:
  - { id: S1, kind: friction, ref: "PostPilot", said: "Rails 7.2.3.2, ~4,700 .rb files, Ruby-aware sf, `languages: [ruby]`, every L0 rule on: NO_CROSS_LAYER_IMPORT 0, ONE_ENTRYPOINT_PER_FILE 0, PERSISTENCE_STAYS_IN_REPOSITORIES 0, EXCEPTIONS_HAVE_ONE_HOME 68 findings" }
  - { id: S2, kind: decision, said: "zero is not clean layering: three rules query for something Rails does not do, and the fourth for something Rails does differently than the model assumes" }
  - { id: S3, kind: code, ref: "catalog/L0/no-cross-layer-import.yaml", said: "the Ruby query (0a285f9) matches require/require_relative whose string names an internal or generated path; Rails autoloads app/ with no import, so it matches nothing there by design" }
  - { id: S4, kind: code, ref: "catalog/L0/no-cross-layer-import.yaml", said: "it fires correctly on remaining lib/, gem and config.ru-level requires; PostPilot's explicit requires never hit an internal/generated path, hence zero" }
  - { id: S5, kind: code, ref: "0a285f9", said: "ONE_ENTRYPOINT_PER_FILE and PERSISTENCE_STAYS_IN_REPOSITORIES ship no Ruby query: config/routes.rb is the Rails idiom, and ActiveRecord models query themselves with no repository layer" }
  - { id: S6, kind: code, ref: "catalog/L0/exceptions-have-one-home.yaml", said: "fires on the Error-suffix convention like Python/TypeScript, but `must_live_in` means one errors.rb per package; Ruby nests the error in its raiser, which no glob expresses; most of the 68 are that idiom" }
  - { id: S7, kind: friction, ref: "graphify of PostPilot at 627b256c", said: "27,031 nodes, 32,101 edges; 9 reverse dependencies into app/controllers, all constant references: 1 from app/models (shopify_store.rb:112), 3 from app/components" }
  - { id: S8, kind: friction, ref: "graphify of PostPilot at 627b256c", said: "5 from app/services: two Expandfi webhook services, a deployment backfill, and campaign_strategies/csv_export_service.rb:20 reaching a controller-owned concern" }
  - { id: S9, kind: friction, ref: "graphify of PostPilot at 627b256c", said: "controllers to models 343 edges, to services 147; 123 of 357 controller files (34%) call a model directly; 10 .erb views call .find_by inline" }
  - { id: S10, kind: friction, ref: "graphify of PostPilot at 627b256c", said: "app/finders (27 files) exists for query logic yet gets 16 inbound edges from services and 10 from controllers, against 595+343 direct model accesses" }
  - { id: S11, kind: friction, ref: "graphify of PostPilot at 627b256c", said: "no path either way between ApplicationConsumer and ApplicationController: the Karafka consumer boundary is real and worth keeping" }
  - { id: S12, kind: decision, said: "a rule reading constant references would have 9 findings on day one, plus a defensible case for widening to the controller/model boundary afterward" }
  - { id: S13, kind: decision, said: "with autoloading the only signal is the constant reference, read against Zeitwerk's path convention: app/services/foo.rb defines Foo; a file under app/services/** referencing a `Controller$` constant is a reverse edge" }
  - { id: S14, kind: code, ref: "src/checks/shape.rs", said: "no resolver or kind needed: EXCEPTIONS_HAVE_ONE_HOME matches `Error$` and constrains the defining node; this matches `Controller$` and constrains the referencing node" }
  - { id: S15, kind: decision, said: "placement: `must_not_live_in: app/services/**, app/models/**, app/components/**`, or symmetrically `must_live_in: app/controllers/**`" }
  - { id: S16, kind: decision, said: "a constant reference is not always a dependency: `FooController.new` and `is_a?` are, one in a comment or string is not; match `(constant)` nodes, not `(string)`" }
  - { id: S17, kind: decision, said: "`const_get(\"FooController\")` and metaprogrammed dispatch are invisible to tree-sitter; PostPilot's webhook services lean on it, so any count is a floor, not a ceiling" }
  - { id: S18, kind: code, ref: "catalog/L1/no-blanket-suppression.yaml", said: "base classes are fan-in: a naive `Controller$` fires everywhere; shape's `unless` cancelling query excludes the framework convention `Application(Controller|Record|Job|Mailer)`, not a per-project value" }
  - { id: S19, kind: decision, said: "three shapes; this plan does not choose: the maintainer decides with these numbers, not inside a bug-fix pull request" }
  - { id: S20, kind: decision, said: "shape 1, widen NO_CROSS_LAYER_IMPORT's Ruby query: cheapest, exists, enabled, has the private/internal/generated vocabulary" }
  - { id: S21, kind: decision, said: "but shape 1 turns 'a require crossing a marked boundary' into 'a reference crossing a layer', defaults `**/_internal/**`, `**/generated/**` meaningless for controllers: same id, other meaning, what 0a285f9 refused" }
  - { id: S22, kind: decision, said: "shape 2, a new Rails L0 rule: own id, defaults protecting app/controllers/**, and its own why: 'controllers are the interface layer; nothing below them reaches back up'" }
  - { id: S23, kind: decision, said: "shape 2's cost: a Ruby-only query in a catalog otherwise portable across five languages, refusing by construction for every language but Ruby" }
  - { id: S24, kind: reference, ref: "templates/client-never-imports-the-data-layer.yaml", said: "shape 3: a parameterised `kind: shape` template with `@@…@@` holes; the Rails form takes the protected directory and suffix (app/controllers/**, Controller$) at sf init" }
  - { id: S25, kind: decision, said: "shape 3 admits the app/finders bypass as a second, separately answerable instance of the same template, not a reason to make the first rule bigger" }
  - { id: S26, kind: decision, said: "the query's `@target` is the constant reference, already in the file `must_not_live_in` checks, as EXCEPTIONS_HAVE_ONE_HOME's `@target` is its class definition: no new placement primitive" }
  - { id: S27, kind: decision, said: "exceptions option 1: a Ruby 'nested inside the raiser' predicate, new shape vocabulary like `must_be_nested_in: <query matching the enclosing class>`, the catalog's first non-glob placement predicate" }
  - { id: S28, kind: decision, said: "exceptions option 2: accept nesting as a home in the rule's Ruby form, as 0a285f9 accepted no Ruby form for two rules; not widening a glob, the one-file model does not describe the idiom" }
  - { id: S29, kind: decision, said: "the options compose: option 2 can ship now (restate or drop the Ruby placement predicate); option 1 is a separate, larger shape capability with its own plan" }
touches:
  - catalog/L0/*.yaml
  - templates/*.yaml
  - interview/**
  - src/checks/shape.rs
  - src/fixtures.rs
  - docs/rules.md
criteria:
  - { id: A1, statement: "the design decision (which of the three shapes, and which id) is made and recorded in `docs/rules.md` before any query ships", proof: "unspecified:this is a decision for the maintainer to make, not something a check can validate", sources: [S19, S20, S21, S22, S23, S24, S25] }
  - { id: A2, statement: "the chosen query fires on `shopify_store.rb:112` or an equivalent fixture of that reverse dependency, and never on any `ApplicationController`/`ApplicationRecord`/`ApplicationJob`/`ApplicationMailer` reference", proof: "deferred:property:src/checks/shape.rs#superclass_read_as_reference", sources: [S2, S3, S5, S13, S14, S15, S16, S18, S26] }
  - { id: A3, statement: "`sf verify` proves the new or widened rule fires on its own mutation fixture in `src/fixtures.rs`, per `L5.EVERY_CHECK_HAS_A_MUTATION_TEST`", proof: "deferred:no fixture is written yet", sources: [S14] }
  - { id: A4, statement: "pointed at PostPilot with `languages: [ruby]`, `sf check` reports at least one of the 9 measured reverse-dependency findings and zero base-class false positives", proof: "deferred:blocked on the query above", sources: [S1, S4, S7, S8, S9, S10, S11, S12, S17] }
  - { id: A5, statement: "EXCEPTIONS_HAVE_ONE_HOME's Ruby `must_live_in` gains a nesting-aware predicate or its `docs/rules.md` entry accepts nesting inside the raiser, so the 68 mostly-idiomatic findings stop reporting as violations", proof: "deferred:depends on which of the two sub-problem options is chosen", sources: [S6, S27, S28, S29] }
  - { id: A6, statement: "no existing rule's `must_live_in`/`must_not_live_in` for another language is touched by this work", proof: "unspecified:an absence, enforced by review of the diff that implements this plan", sources: [S21, S28] }
crosses:
  - { mechanism: catalog, answer: works, proof: "deferred:A3" }
  - { mechanism: mutation-fixtures, answer: works, proof: "deferred:A3" }
  - { mechanism: generated-docs, answer: works, proof: "deferred:A1, A5 write to docs/rules.md, whose rule sections render from the catalog and are checked" }
  - { mechanism: init-and-interview, answer: works, proof: "deferred:only if A1 picks shape 3, the interview-instantiated template" }
  - { mechanism: languages, answer: works, proof: "deferred:A5, only if exceptions option 1's nesting predicate is chosen; the cross-layer query needs no new primitive (S26)" }
guarantee:
  - { holds: "a cross-layer finding is a real constant reference from a lower layer into app/controllers", not_against: "const_get and metaprogrammed dispatch: the count is a floor (S17)" }
cut:
  - "choosing among the three shapes in this plan: left to the maintainer (S19)"
  - "a cross-file symbol resolver or a new check kind: not needed (S14)"
  - "widening the errors glob to make findings disappear: refused (S28)"
  - "exceptions option 1 inside this work: a separate plan (S29)"
exit: "`sf check --changed` on a Rails repo with `languages: [ruby]` reports ≥1 of PostPilot's 9 reverse-dependency sites (S7-S8), none on Application(Controller|Record|Job|Mailer) inheritance anywhere there, and EXCEPTIONS_HAVE_ONE_HOME's Ruby form fires only on misplaced exceptions or `docs/rules.md` says why nesting is accepted"
---
# The four L0 structural rules assume an import statement

On a 4,700-file Rails app three L0 rules report zero and the fourth reports 68 mostly idiomatic findings, while a knowledge graph finds 9 real reverse dependencies: Rails autoloads constants, so there is no import statement to match.
