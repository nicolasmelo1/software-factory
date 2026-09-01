# Rules activate on the version of the dependency they are about

A deprecation rule for Tailwind 3, or a shape rule for the QuickBooks v3 API,
is only correct while that version is the one installed. Today a rule instance
is on or off and nothing connects it to the manifest, so the correctness of the
policy depends on somebody remembering.

Two failure modes. The second is the expensive one.

**A rule for a version you no longer run.** The pin moves to Tailwind 4, the v3
rules keep firing on code that is now right, and the cheapest way to green is to
widen an exclusion. That is the exact move `L2.POLICY_ONLY_TIGHTENS` exists to
make visible, so it is noisy rather than dangerous.

**A rule that quietly stops applying.** The pin moves, the patterns no longer
match anything, and the rule reports green forever. `L5.NO_INERT_RULE` catches
structural inertness today, an empty lock scope, a command with no `run`, a
query for no declared language, but it cannot catch a `text_pattern` rule whose
regexes describe an API nobody calls anymore. It reads exactly like a rule that
is protecting you.

## The shape

```yaml
rules:
  # PACK_RULE_ID stands in for the pack's own id. A plan cannot name an
  # unwritten rule: `L4.EVERY_RULE_HAS_A_WHY` reads this file too.
  PACK_RULE_ID@tailwind3:
    enabled: true
    when:
      dependency: tailwindcss
      manifest: package.json
      version: "^3"
```

The version comes from the manifest, which is already inside the scope of
`L2.DEPENDENCIES_CHANGE_DELIBERATELY` (see
[`.software-factory/policy.yaml`](../.software-factory/policy.yaml)). That is
what makes the condition trustworthy: the input cannot move without a lock
update in the same commit, so `when` is gated by a rule that already exists
rather than being a new thing to trust.

## Decisions

**A `when` that does not match is a finding, not a skip.** This is the whole
point. A rule whose condition went false announces that it no longer applies
and names the version it expected against the version found, so the answer is
to remove it or repoint it. Silently disabling itself is how a policy becomes
decoration, and it would hand an agent a way to turn a rule off by editing a
dependency. The finding belongs in `L5.NO_INERT_RULE`, whose vocabulary this
already is.

**The manifest range, not the resolved lock version.** The lock is more
accurate and there are four lock formats per ecosystem, each with its own
schema and its own churn. The manifest range is what the team decided, and the
decision is what the rule is about.

**Declared dependencies only.** A `when` naming a package no manifest declares
is a finding, same as a condition that went false. A rule about a transitive
dependency is a rule about somebody else's choice, and it will break on a
resolution nobody made.

## What this does not do

It does not check that the rule's content is right for the version it claims.
A regex written for Tailwind 3 stays a regex written for Tailwind 3 whether or
not the condition matches. Reading the upstream changelog and turning it into
patterns is a once-per-bump job for a model, not something CI can decide, and
it is the job [`factory-author`](../skills/factory-author/SKILL.md) exists for.

**Exit condition:** a repository pinned to a major version runs that version's
rules, and changing the pin in the manifest makes `sf check` fail by naming
every rule whose `when` no longer matches, instead of leaving them enabled and
inert.

## Acceptance criteria

- [x] `when: {dependency, manifest, version}` parses in the policy and decides
      whether a rule instance runs
      (proof: test:src/policy.rs)
- [x] A rule instance whose `when` no longer matches produces a finding naming
      the expected range and the version found
      (proof: test:.software-factory/mutations/L5.NO_INERT_RULE/)
- [x] A `when` naming a dependency no manifest declares is a finding, not a
      skip (proof: test:.software-factory/mutations/L5.NO_INERT_RULE/)
- [x] `sf verify` still proves every conditional rule fires, with the fixture
      carrying the manifest that satisfies its condition
      (proof: test:src/verify.rs)
- [x] Nothing reads a lock file to resolve a version
      (proof: unspecified:an absence is not checkable, it is a review note on
      the diff that adds `when`)
