# Every fixture carries its repair

The escape log shipped in `703f899`. [`src/escapes.rs`](../src/escapes.rs)
records working-tree attempts against a finding key, minimises an escape to the
hunks that flip that key, classifies a hunk touching the policy, the ratchet or
a lock as a weakening rather than advice, and the report grows the trail past
the attempt ceiling. Twenty-one tests hold it.

The cold start did not ship, and it was the part that made the log useful on
day one.

## Measured

`Fixture` in [`src/fixtures.rs`](../src/fixtures.rs) carries four fields: the
rule, two policy fragments and the files. None of them is a repair. Forty
fixtures, zero repairs. `sf verify` asserts that the mutation trips its rule
and asserts nothing about clearing it. So retrieval's last resort, the rule's
own worked example, resolves to nothing for every rule in the catalog, and the
log stays empty until somebody in this repository has personally been stuck on
that rule.

The argument the escape-log plan made for this is unchanged: an escape log that
only accumulates learns from whoever was stuck first, and a proven repair
beside every proven mutation is the same guarantee arriving by proof instead of
by accident. Every check has a mutation that proves it fires and a repair that
proves it clears.

## The assertion that exists nowhere

That plan also promised `assertion:escape.breaks_a_real_loop`: a model looping
on a real finding in this repository gets out with the log where it did not
without it. No gate requires that assertion and no evidence manifest carries
it. A gate assertion that exists nowhere is a criterion that cannot go green,
so it is either a required assertion on a gate, with a harness that produces
it, or it is not a criterion. That decision belongs to whoever picks this up,
and it is cheaper than it looks, because the repair fixtures are the corpus a
loop test would need.

## Acceptance criteria

- [ ] `Fixture` carries a repair, and `sf verify` fails both when the mutation
      does not trip the rule and when the repair does not clear it
      (proof: test:src/verify.rs)
- [ ] Every fixture carries one, in every language the fixture claims, so no
      rule ships with a before and no after
      (proof: test:src/fixtures.rs)
- [ ] Retrieval falls back to the rule's repair when no local escape ranks, so
      a rule nobody has been stuck on still renders a worked example
      (proof: test:src/escapes.rs)
- [ ] Whether "a model looping on a real finding gets out" is a required
      assertion on a gate or is not a criterion is decided before the repairs
      land
      (proof: unspecified:it asks what evidence a gate should require, and no
      check answers that)
- [ ] A recurring escape is promoted into the rule's `fix` prose and deleted
      (proof: deferred:needs the log to have run long enough to have produced
      a recurring entry)

**Exit condition:** `sf verify` reports, for every enabled rule, both that its
mutation trips the rule and that its repair clears it, and an `sf check`
finding on a rule nobody in this repository has ever been stuck on renders a
worked example that came out of the binary rather than out of somebody's
working tree.
