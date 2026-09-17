# A property is a proof with a falsifier

A criterion today names one of four things: `assertion:`, `test:`, `deferred:`
or `unspecified:` (`src/checks/cadence.rs:732`). Three of those are honest
about what they are. The fourth hides a distinction that matters more than any
of them: `test:tests/foo.rs` says somebody checked *an example*, and nothing in
the vocabulary can say somebody checked *the statement*.

That gap is where the method's own sentence stops being enforceable. "Every
rule that matters is written twice — once as prose that says why, once as a
check that fails" is about the catalog. A product has rules too, and they are
written once, in prose, in a plan: *a retry that produces no new evidence is
refused*, *the counter stays honest*, *reconciliation is idempotent*. Each of
those is a statement over all inputs, and each is currently proven by three
examples somebody thought of on the day.

## The formal version is out of reach and the portable one is not

Stating a law over all inputs and proving it is what dependent type theories
do. It is also not available to any repository that is not written in one, and
translating a mainstream language into one is a decade of work per language
that rots with the language's own specification.

The portable version already exists in every language this tool parses:
`proptest` in Rust, `Hypothesis` in Python, `fast-check` in TypeScript,
`gopter` in Go, `rantly` in Ruby. They do not prove anything. They generate,
they shrink, and they report the smallest input that breaks the statement. That
is weaker than a proof and stronger than an example, and the vocabulary has no
word for it.

## A marker that only renames `test:` is decoration

This is the part worth getting right, because the cheap version of this plan is
worthless. Adding `property:` as a synonym would let a plan read as if it had
proven a statement while running the same three examples, which is a new way to
be green and say nothing — the failure this catalog exists to refuse.

So the kind carries a second obligation, and it is the same obligation L5 puts
on the catalog's own rules. `L5.EVERY_CHECK_HAS_A_MUTATION_TEST` refuses a rule
that nothing proves fires. A property nothing falsifies is the same object one
level out: a statement that holds because it is weaker than the author thought,
passing forever, reading exactly like one that is protecting something.

A property therefore names its falsifier — the mutation of the implementation
that the property must reject — and the check runs both directions, as `verify`
already does: the mutant is refused, and the unmutated code passes.

## What changes

- `PROOF_KINDS` gains `property`, and the marker takes a path and a falsifier:
  `(proof: property:PATH#FALSIFIER)`.
- A new L5 rule reads those criteria and asserts each named falsifier exists
  and is exercised. A `property:` with no falsifier is a finding, not a pass.
- `L4.PLAN_PROOF_BUDGET` counts `property:` as proven, because it is — the
  budget measures what nobody worked out how to check, and this is checked.

## What this is not

It is not a proof, and the vocabulary must not let it read as one. A statement
that survives ten thousand generated cases is evidence about a generator, not a
theorem about a domain: the generator's distribution is a judgement nobody
audits, and shrinking finds the smallest failing case rather than all of them.
That is why the kind is `property` and not `law`. A repository that wants the
stronger word can have it when it is written in a language that can earn it.

The first two consumers are already pure and already documented as such: a
workflow's decision function, which is a fold from a record and an observation
to a plan, and this tool's own `L2.POLICY_ONLY_TIGHTENS`, which is
monotonicity over a policy — the direction of a change, not its content. Both
are total functions over data, which is the only shape this technique is honest
about.

## Acceptance criteria

- [ ] A criterion marked `(proof: property:PATH#FALSIFIER)` parses, and one
      missing either half is refused with the vocabulary in the message
      (proof: test:src/checks/cadence.rs)
- [ ] A `property:` criterion whose named falsifier does not exist is a
      finding, and the same criterion with it present passes
      (proof: test:src/checks/properties.rs)
- [ ] A falsifier that the property fails to reject is a finding, so a
      property that cannot fail cannot be cited
      (proof: test:src/checks/properties.rs)
- [ ] `L4.PLAN_PROOF_BUDGET` counts a `property:` criterion as proven rather
      than as debt
      (proof: test:src/checks/cadence.rs)
- [ ] The rule ships with a fixture that trips it and a repair that clears it,
      and `sf verify` proves both
      (proof: test:src/fixtures.rs)
- [ ] This repository's own `L2.POLICY_ONLY_TIGHTENS` carries a property with
      a falsifier, and the criterion citing it is green
      (proof: unspecified:the property is written against the policy merge, and
      which statement is worth generating over is a judgement this plan should
      not pre-empt)

**Exit condition:** a plan in a repository nobody tuned this for cites a
statement rather than an example, the citation names the mutation that statement
rejects, and removing that mutation turns the plan's own gate red — so a
property that stopped being able to fail stops counting as proof.
