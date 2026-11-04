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
  - { id: A1, statement: "a criterion marked `(proof: property:PATH#FALSIFIER)` parses; one missing either half is refused with the vocabulary in the message", proof: "test:src/checks/cadence.rs", sources: [S1, S2, S3, S8] }
  - { id: A2, statement: "a `property:` criterion whose named falsifier does not exist is a finding; with it present it passes", proof: "test:src/checks/properties.rs", sources: [S5, S9] }
  - { id: A3, statement: "a falsifier the property fails to reject is a finding, so a property that cannot fail cannot be cited", proof: "test:src/checks/properties.rs", sources: [S4, S6, S7, S11] }
  - { id: A4, statement: "`L4.PLAN_PROOF_BUDGET` counts a `property:` criterion as proven rather than as debt", proof: "test:src/checks/cadence.rs", sources: [S10] }
  - { id: A5, statement: "the rule ships with a fixture that trips it and a repair that clears it, and `sf verify` proves both", proof: "test:src/fixtures.rs", sources: [S6, S9] }
  - { id: A6, statement: "this repository's own `L2.POLICY_ONLY_TIGHTENS` carries a property with a falsifier, and the criterion citing it is green", proof: "unspecified:the property is written against the policy merge, and which statement is worth generating over is a judgement this plan should not pre-empt", sources: [S12] }
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
exit: "a plan in a repository nobody tuned this for cites a statement rather than an example, the citation names the mutation that statement rejects, and removing that mutation turns the plan's own gate red"
---
# A property is a proof with a falsifier

A product's plan rules are statements over all inputs, proven today by three examples, and the proof vocabulary cannot say otherwise. A property that stopped being able to fail must stop counting as proof.
