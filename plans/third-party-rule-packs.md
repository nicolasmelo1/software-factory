# Rule packs for third-party APIs and libraries

The two rules that already carry third-party conformance are
`L2.DEPENDENCIES_CHANGE_DELIBERATELY`, which pins the version, and
`L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE`, which regenerates a client from a
vendored spec and diffs the result. Between them they cover everything a vendor
publishes in machine-readable form: an OpenAPI document, a WSDL, a `.d.ts`.

What they do not cover is everything a vendor publishes as prose. A removed
utility class, an endpoint that still responds but is documented as going away,
an argument whose meaning changed. That knowledge exists once, in a migration
guide, and today every team that needs it re-derives it by reading.

A pack is that reading, written down once as rules, versioned against the
upstream release it describes, and installed:

```sh
sf pack add tailwind@3
```

The model reads the changelog once and emits catalog-shaped YAML. CI then runs
Rust, with no network and no model. The pack is the durable artifact, which is
the only reason this is worth building: it moves the cost from every run to
every upgrade.

## The three hard parts

**Trust.** A pack is data that a `kind: command` rule can turn into execution.
Installing one from a URL is installing a script. Packs are either vendored
into the repository at install time, where they sit in the diff and under
`L2.FACTORY_CONFIG_IS_LOCKED` like every other rule file, or they are not
worth having.

**Proof.** `L5.EVERY_CHECK_HAS_A_MUTATION_TEST` says an enabled rule with
nothing proving it fires is not a rule. A pack of forty rules and no fixtures
is forty rules that report green and nobody has ever seen fail. Install must
refuse a pack whose rules do not trip their own fixtures, which means `sf pack
add` runs `sf verify` over the pack before writing anything.

**Expiry.** A pack for a version you no longer run has to say so out loud, and
that is what [rules-activate-by-dependency-version.md](rules-activate-by-dependency-version.md)
built, shipped in `927e8e2`. Without it, every pack is something to remember
to remove.

## Deliberately not in scope

Fetching the vendor's current spec to see whether it moved. `sf check` does not
touch the network, on purpose: a policy file travels with a clone. Comparing a
vendored spec against upstream is a scheduled job with `--allow-commands`, and
its output is a pull request that bumps the pack, not a red PR build.

**Exit condition:** `sf pack add <name>@<version>` vendors a versioned set of
rules with their fixtures into `.software-factory/rules/`, refuses any pack
whose fixtures do not trip its own rules, and the installed rules deactivate
with a finding when the dependency's major version moves.

## Acceptance criteria

- [ ] A pack is catalog-shaped rule YAML plus one mutation fixture per rule,
      with no new format to learn
      (proof: deferred:the catalog format is fixed, the packaging around it is
      not designed yet)
- [ ] `sf pack add` runs `sf verify` over the pack and writes nothing if any
      rule fails to fire
      (proof: deferred:depends on the install path, not designed yet)
- [ ] Installed pack rules carry a `when` naming the dependency and version
      they describe
      (proof: deferred:the pack format carries no `when` field yet)
- [ ] No pack is fetched or evaluated at check time
      (proof: unspecified:an absence, enforced by review of the diff that adds
      the subcommand)
