---
sources:
  - { id: S1, kind: code, ref: "catalog/L2/", said: "L2.DEPENDENCIES_CHANGE_DELIBERATELY pins the version; L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE regenerates a client from a vendored spec and diffs: together they cover machine-readable vendor output (OpenAPI, WSDL, `.d.ts`)" }
  - { id: S2, kind: friction, said: "prose-only vendor knowledge (a removed utility class, a still-responding endpoint documented as going, an argument whose meaning changed) lives once in a migration guide, and every team re-derives it" }
  - { id: S3, kind: decision, said: "a pack is that reading written once as rules, versioned against the upstream release it describes, and installed: `sf pack add tailwind@3`" }
  - { id: S4, kind: decision, said: "a model reads the changelog once and emits catalog-shaped YAML; CI runs Rust with no network and no model; cost moves from every run to every upgrade" }
  - { id: S5, kind: decision, said: "a pack is data a `kind: command` rule can turn into execution; installing from a URL installs a script, so packs are vendored at install time, in the diff, under L2.FACTORY_CONFIG_IS_LOCKED" }
  - { id: S6, kind: code, ref: "c8a1db6", said: "`sf check --policy` governs a repository from outside; it is a read path, never an install path for a pack" }
  - { id: S7, kind: decision, said: "per L5.EVERY_CHECK_HAS_A_MUTATION_TEST, install refuses a pack whose rules do not trip their own fixtures: `sf pack add` runs `sf verify` over the pack before writing anything" }
  - { id: S8, kind: reference, ref: "docs/design/rules-activate-by-dependency-version.md", said: "`when` conditions, shipped in 927e8e2, let a pack for a version you no longer run say so out loud, instead of being something to remember to remove" }
touches:
  - src/main.rs
  - src/pack.rs
  - src/verify.rs
criteria:
  - { id: A1, statement: "a pack is catalog-shaped rule YAML plus one mutation fixture per rule, with no new format to learn", proof: "deferred:the catalog format is fixed, the packaging around it is not designed yet", sources: [S1, S2, S3, S4] }
  - { id: A2, statement: "`sf pack add` runs `sf verify` over the pack and writes nothing if any rule fails to fire", proof: "deferred:property:src/pack.rs#writes_each_rule_as_it_verifies", sources: [S5, S7] }
  - { id: A3, statement: "installed pack rules carry a `when` naming the dependency and version they describe", proof: "deferred:the pack format carries no `when` field yet", sources: [S8] }
  - { id: A4, statement: "no pack is fetched or evaluated at check time", proof: "unspecified:an absence, enforced by review of the diff that adds the subcommand", sources: [S4, S6] }
crosses:
  - { mechanism: commands, answer: works, proof: "deferred:A2" }
  - { mechanism: mutation-fixtures, answer: works, proof: "deferred:A2" }
  - { mechanism: locks-and-ratchet, answer: works, proof: "deferred:vendored pack rules sit under L2.FACTORY_CONFIG_IS_LOCKED like every rule file (S5)" }
  - { mechanism: overlay, answer: refused, by: "`sf check --policy` is a read path, never an install path for a pack (S6)" }
failures:
  - { step: "sf pack add", when: "any pack rule fails to trip its fixture", goes_to: "refused; nothing is written" }
cut:
  - "installing a pack from a URL: refused, it is installing a script (S5)"
  - "fetching the vendor's current spec at check time: sf check never touches the network, since a policy travels with a clone"
  - "comparing a vendored spec against upstream: a scheduled `--allow-commands` job whose output is a PR bumping the pack, not a red build"
exit: "`sf pack add <name>@<version>` vendors a versioned set of rules with their fixtures into `.software-factory/rules/`, refuses any pack whose fixtures do not trip its own rules, and the installed rules deactivate with a finding when the dependency's major version moves"
---
# Rule packs for third-party APIs and libraries

What a vendor publishes as prose (removals, deprecations, changed meanings) exists once in a migration guide and is re-derived by every team. A pack writes that reading down once, as proven rules.
