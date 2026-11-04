---
base: 6914586d0c97e6185c798717b364f6d7c3c914ae
sources:
  - { id: S1, kind: code, ref: "src/checks/cadence.rs", said: "PROOF_KINDS is assertion, test, deferred, unspecified; `test:` says an example was checked, and nothing can say the statement was" }
  - { id: S2, kind: reference, ref: "docs/method.md", said: "every rule is written twice, prose and a failing check; a product's plan rules (no-evidence retry refused, counter honest, reconciliation idempotent) are statements over all inputs proven by three examples" }
  - { id: S3, kind: decision, said: "proving laws over all inputs needs a dependent type theory; translating a mainstream language is a decade per language and rots, so it is out of reach" }
  - { id: S4, kind: reference, said: "proptest (Rust), Hypothesis (Python), fast-check (TypeScript), gopter (Go), rantly (Ruby) generate, shrink and report the smallest breaking input: weaker than proof, stronger than example" }
  - { id: S5, kind: decision, said: "`property:` as a synonym of `test:` is decoration, green while saying nothing; the kind carries a falsifier obligation" }
  - { id: S6, kind: code, ref: "catalog/L5/every-check-has-a-mutation-test.yaml", said: "L5.EVERY_CHECK_HAS_A_MUTATION_TEST refuses a rule nothing proves fires; an unfalsified property is the same object one level out" }
  - { id: S7, kind: decision, said: "a property names its falsifier, the implementation mutation it must reject; the check runs both directions like verify: mutant refused, unmutated code passes" }
  - { id: S8, kind: decision, said: "PROOF_KINDS gains `property`; the marker is `(proof: property:PATH#FALSIFIER)`" }
  - { id: S9, kind: decision, said: "a new L5 rule asserts each named falsifier exists and is exercised; a `property:` with no falsifier is a finding, not a pass" }
  - { id: S10, kind: decision, said: "L4.PLAN_PROOF_BUDGET counts `property:` as proven: the budget measures what nobody worked out how to check, and this is checked" }
  - { id: S11, kind: decision, said: "the kind is `property`, not `law`: surviving generated cases is evidence about a generator, whose distribution nobody audits; shrinking finds the smallest failing case, not all" }
  - { id: S12, kind: decision, said: "first consumers are pure total functions over data: a workflow's decision function (record + observation to plan) and L2.POLICY_ONLY_TIGHTENS (monotonicity over a policy)" }
touches:
  - src/checks/cadence.rs
  - src/checks/properties.rs
  - src/catalog.rs
  - catalog/L5/*.yaml
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "a criterion marked `(proof: property:PATH#FALSIFIER)` parses; one missing either half is refused with the vocabulary in the message", proof: "test:src/checks/cadence.rs", sources: [S1, S2, S3, S8], done: true }
  - { id: A2, statement: "a `property:` criterion whose named falsifier does not exist is a finding; with it present it passes", proof: "test:src/checks/properties.rs", sources: [S5, S9], done: true }
  - { id: A3, statement: "a falsifier the property fails to reject is a finding, so a property that cannot fail cannot be cited", proof: "test:src/checks/properties.rs", sources: [S4, S6, S7, S11], done: true }
  - { id: A4, statement: "`L4.PLAN_PROOF_BUDGET` counts a `property:` criterion as proven rather than as debt", proof: "test:src/checks/cadence.rs", sources: [S10], done: true }
  - { id: A5, statement: "the rule ships with a fixture that trips it and a repair that clears it, and `sf verify` proves both", proof: "test:src/fixtures.rs", sources: [S6, S9], done: true }
  - { id: A6, statement: "this repository's own `L2.POLICY_ONLY_TIGHTENS` carries a property with a falsifier, and the criterion citing it is green", proof: "property:src/checks/tightening.rs#falsy_read_as_unset", sources: [S12], done: true }
crosses:
  - { mechanism: plans, answer: works, proof: "test:src/checks/cadence.rs (A1, A4)" }
  - { mechanism: inertness, answer: n/a, why: "cadence.rs gains a proof kind; no rule's inert reporting changes" }
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A5)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A5)" }
guarantee:
  - { holds: "a cited property rejects its named mutation and passes the unmutated code", not_against: "a generator's unaudited distribution, or failing cases shrinking never reaches" }
cut:
  - "formal proof via dependent types, and a `law` kind: out of reach for mainstream languages (S3, S11)"
  - "`property:` as a renamed `test:`: refused (S5)"
deviations:
  - "A6 is built rather than left unspecified: the property is over `rule_findings` — any weakening move from any explicit starting value is reported, its inverse is silent — and its falsifier is a merge reading an explicit 0, false or empty override as unset"
  - "A6 needs a seam: `rule_findings` and `effective_size` take the merge as a parameter, so the property can plant the falsifier; `proptest` joins as a dev-dependency (std only, no regression file written), touching Cargo.toml and Cargo.lock"
  - "how a falsifier is exercised, the same in every language: the rule's `run` command runs with SF_PROPERTY set to PATH, once as is (exit 0) and once with SF_FALSIFIER set (non-zero); the property swaps its mutant in when it reads the name"
  - "exists means the falsifier is a word in PATH: checked before anything runs, so A2 holds without --allow-commands; exercised runs under `sf check --allow-commands`, and without it each property is reported not-run, as a command rule is"
  - "the rule is L5.EVERY_PROPERTY_HAS_A_FALSIFIER, a cadence mode `property_falsifiers` registered in src/checks/mod.rs, which is not in touches; under an overlay it runs, beside the plan and code it reads"
  - "A5: `sf verify` proves the fixture trips the rule; the repair clearing it is proven by the unit test in src/fixtures.rs, since a fixture's own `fixed:` half is row 6"
  - "L3.GATE_COVERS_THE_PLAN counts a property as an undeferred proof, and L4.PLAN_CRITERION_NAMES_ITS_CHECK names the form in its statement, fix and refusal"
  - "this repository enables the rule over plans/ and docs/design/, running the cited file's `property` test module; docs/method.md and the factory-author skill name the kind"
  - "`sf ready` still reads only `test:` paths; a `property:` path it does not check is left for when ready.rs is next touched"
exit: "a plan in a repository nobody tuned this for cites a statement rather than an example, the citation names the mutation that statement rejects, and removing that mutation turns the plan's own gate red"
---
# A property is a proof with a falsifier

Delivered. This document was `plans/a-property-is-a-proof-with-a-falsifier.md`, the plan this repository executed for it; it now lives here as the design note, with every criterion checked and what the work found under `deviations:`.

A product's plan rules are statements over all inputs, proven today by three examples, and the proof vocabulary cannot say otherwise. A property that stopped being able to fail must stop counting as proof.
