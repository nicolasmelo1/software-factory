# Stopping an agent from relaxing the rules

Every rule here is enforced by a file an agent can edit. The shortest path from
a red build to a green one is not fixing the code — it is disabling the rule,
widening a glob, or deleting a workflow step, and at the diff level all three
are indistinguishable from a fix. Two rules close that door.

**`L2.FACTORY_CONFIG_IS_LOCKED`** hash-locks the policy, the ratchet, the local
rules, the root allowlist, the CI workflow and the hooks. Editing any of them
without `sf lock` in the same commit fails. The lock does not make the edit
impossible — it makes it undeniable, as a second deliberate line in the diff on
a path a code owner watches.

**`L2.POLICY_ONLY_TIGHTENS`** reads the edit and decides which direction it
went:

```sh
sf check --changed origin/main
```

A rule disabled or removed, an exclusion added, a scope narrowed, a ceiling
raised, a gate weakened, a new violation frozen, a review date pushed out — all
fail. Tightening passes silently, so the rule never taxes the direction you
want.

Alongside them, `.allowed-root-files` blocks the `NOTES.md` / `PLAN.md` reflex,
the dependency lock turns adding a package into a reviewable act, and
`L2.NO_PERMANENT_EXCEPTION` fails the build when a frozen exception outlives its
review date.
