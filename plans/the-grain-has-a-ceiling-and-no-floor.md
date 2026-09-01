# The grain has a ceiling and no floor

The catalog ships 34 rules across seven layers. None of them is about excess
indirection.

L1 is the layer about how the code reads, and it holds four rules: a
complexity ceiling, a ban on the untyped escape hatch, a ban on blanket
suppression, and a skipped test that states a reason. All four point the same
way. They charge for too much in one place, and never for too little spread
across too many.

[`catalog/L1/complexity-ceiling.yaml`](../catalog/L1/complexity-ceiling.yaml)
states the direction as its justification: "The ceiling makes the refactor the
cheaper option." [`docs/method.md`](../docs/method.md) names the agent
behaviour it counters: "appending a branch is a smaller diff than a refactor."
Both are correct. Neither has anything on the other side of it.

Measured in this repository. `cb9ac0c` extracted
`inert_toolchain_reason` out of `inert_rules` in `src/checks/cadence.rs`, and
its own pull request said why: so the new branch "pays into its own
`L1.COMPLEXITY_CEILING` budget instead of pushing `inert_rules` itself over the
ceiling". That extraction was the right call. It is also a check in the
catalog producing a new function, with nothing in the catalog able to ask
whether the next one is still earning its name.

## What has no representation today

Two of the most common failure modes in agent-written code, assuming the code
works:

- unnecessary logical layers and abstractions, reached for by default
- too many files and modules for the work being done

They are one property from two angles: indirection that costs a reader a hop
and returns nothing for it.

## Why this is a design question and not a missing YAML file

Every `sf` finding has a site. A path, a line, a stable key, an expected and an
actual, and a ratchet entry that can freeze it. "Fourteen layers where three
would do" has no site, which is why no existing `kind` can express it and why
this plan does not pick a rule id.

The useful half of the problem is that some of the excess does have a site:

- a function whose body forwards to exactly one other call
- an abstraction with exactly one implementer
- an indirection chain deeper than N hops between an entrypoint and its effect
- a directory of many files each holding almost nothing

Those can be pointed at. Aggregate taste cannot, and refusing it is better
than approximating it, the same way the Ruby language adapter (`0a285f9`)
refused to port `L0.PERSISTENCE_STAYS_IN_REPOSITORIES` rather than invent a
Rails meaning for it.

## False-positive risk, measured

The cheapest candidate is the one-line forwarder, and it was run over this
repository at `8e22ce5`: 6,392 lines, 26 files, 231 functions, 246 lines per
file, 8.9 functions per file. It produces 18 hits, and all 18 were read, not
sampled. Every one is a named accessor whose name is the value:

```
src/catalog.rs:310   fn get           -> self.rules.get(id)
src/policy.rs:238    fn base_rule_id  -> key.split('@').next().unwrap_or(key)
src/ratchet.rs:61    fn allows        -> self.rules.get(rule).is_some_and(..)
src/ratchet.rs:118   fn frozen        -> keys.iter().map(|k| k.to_string())..
src/digest.rs:13     fn file          -> Ok(hex(&std::fs::read(path)?))
src/report.rs:22     fn json          -> Ok(serde_json::to_string_pretty(self)?)
src/checks/cadence.rs:691 fn joined   -> names.cloned().collect::<Vec<_>>()..
src/checks/shape.rs:190 fn trim_quotes -> s.trim_matches(|c| c == '"' || ..)
```

The remaining ten are the same shape: `checks::evidence::load` and `::fail`,
`fixtures::for_rule`, `interview::get`, three `*_dir` constants built from
`CARGO_MANIFEST_DIR`, `main::long_flags`, `policy::rules_document`.

Not one is slop. This is not a threshold that needs tuning. The ceiling rule's
own `fix:` says "Extract the branch cluster into a named function. Naming it is
the point." A floor rule that charges for a small named function contradicts
the stated justification of the only other rule in its own layer. Any floor
whose unit is size is dead before it ships.

Second measurement, same repository and same sha: `src/` defines zero traits.
There is no abstraction layer here whose implementers could be counted. So
this repository is a usable negative control for a floor rule and a useless
positive one. The corpus has to come from elsewhere, and that is a
precondition of the work, not a detail of it.

## Candidate shapes

Three, undecided on purpose. The choice is about the finding model, which is
the maintainer's call and not something to fold into whichever diff ships
first.

1. **`kind: command`, the sanctioned extension path**
   ([`README.md`](../README.md)). A script computes the metric and a nonzero
   exit is the finding. Ships without touching the binary, and per-area
   calibration already exists through the `RULE@name` policy key. The cost is
   the whole difficulty stated plainly: the finding has no site, so the ratchet
   cannot freeze one violation at a time and the report cannot say where. An
   adopting repository gets a number it must take on faith, which is the shape
   of evidence this method exists to reject.
2. **A new structural kind for depth.** Site-bearing, and it says the thing
   worth saying. It needs cross-file symbol resolution, which no kind has
   today: `shape` and `nested` are per-file tree-sitter queries. This is the
   largest of the three by a wide margin, and it is the one that would also
   serve [the four L0 structural rules assume an import
   statement](structural-rules-assume-an-import-statement.md), which wants a
   query reading a constant reference against a path convention for the same
   reason: the signal it needs is not in the file the finding lands in.
3. **A `shape` rule for the single-file subset only.** No new kind, no
   resolver: a forwarder whose target is declared in the same file, an
   abstraction declared and implemented once in the same file. Smallest honest
   thing that can ship, and it deliberately sees less than the problem. Worth
   preferring only if the measured corpus shows the single-file subset is where
   the excess actually lives, which nobody has checked.

## Non-goals

Test volume is out of scope. Test efficacy already has a rule in
`L5.EVERY_CHECK_HAS_A_MUTATION_TEST`; volume has no site either, and arguing
it here would import a second undecided question into this one.

The inverse pressure in L6 is out of scope. Seven of the nine L6 rules are
`kind: toolchain`, presence checks whose only possible remedy is adding another
tool to CI. Whether that layer should stop growing is a decision, not a rule,
and it does not belong in a plan about L1.

`L1.COMPLEXITY_CEILING` is not touched, and neither is its `max: 12` default or
any ceiling in a policy. A floor argued as grounds for raising the ceiling
would be the silent loosening `L2.POLICY_ONLY_TIGHTENS` exists to stop, arriving
under cover of a plan that sounded like the opposite.

## Acceptance criteria

- [ ] The choice among the three shapes above is made and recorded in
      `docs/rules.md` before any check ships
      (proof: unspecified:this is a decision about the finding model, which no
      check can validate)
- [ ] A positive corpus exists: at least one repository, named with a commit
      sha, where the chosen metric points at indirection that repository's own
      maintainer agrees is excess
      (proof: deferred:no corpus is measured yet, and this repository cannot
      serve as one at 8.9 functions per file and zero traits)
- [ ] The chosen check reports zero findings on `software-factory`'s own `src/`,
      with the 18 named accessors measured above and the `cb9ac0c` extraction
      all silent
      (proof: deferred:no check is written yet)
- [ ] `sf verify` proves the new rule fires on its own mutation fixture in
      `src/fixtures.rs`, per `L5.EVERY_CHECK_HAS_A_MUTATION_TEST`
      (proof: deferred:no fixture is written yet)
- [ ] `L5.NO_INERT_RULE` can see the new rule's inertness path
      (proof: deferred:depends on the kind chosen, since `kind: command` is
      already covered and a new kind would not be)
- [ ] No existing ceiling, in the catalog or in any policy, moves as part of
      this work
      (proof: unspecified:an absence, enforced by review of the diff, and by
      `L2.POLICY_ONLY_TIGHTENS` if a policy ceiling is touched)

**Exit condition:** `sf check` on a repository nobody tuned it for names at
least one indirection site whose own maintainer agrees should not exist, and
reports zero findings on `software-factory`'s `src/`, including the
`inert_toolchain_reason` extraction in `cb9ac0c` and all 18 named accessors
measured above, every one of which is correct code.
