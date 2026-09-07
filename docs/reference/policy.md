# Pointing the policy at your repository

One rule, several settings. Several checkouts, one policy. And the escape
hatch for a check this tool cannot express.

## One rule, different settings per package

A monorepo needs the same rule twice: a complexity ceiling of 12 in the new
packages and 20 in the one nobody has had time to split up. Write the policy
key as `RULE@name`:

```yaml
rules:
  L1.COMPLEXITY_CEILING:
    enabled: true
    options: {max: 12, scope: ["packages/api/**", "packages/web/**"]}

  L1.COMPLEXITY_CEILING@legacy:
    enabled: true
    options: {max: 20, scope: ["packages/legacy/**"]}
```

Both resolve to one catalog rule with one written reason, and each instance
gets its own findings and its own ratchet entries — so paying down the legacy
debt does not require touching the other packages' allowance.

## Repositories that are checked together

Some invariants only exist *between* repositories: a public contract and the
private service that serves it, a schema and the client generated from it.
Neither checkout can see the other, so nothing in either notices when they
drift.

Declare the other checkouts — usually symlinks to sibling clones — and one
policy governs all of them:

```yaml
project:
  name: acme-workspace
  languages: [python, typescript]
  roots: ["acme-public", "acme-private"]
```

Findings keep the declared prefix (`acme-private/packages/api/handler.py`), so
a rule reads the same wherever the checkout actually lives. Symlinks are
followed **only** for declared roots, never during the ordinary walk — a
package manager's symlink farm would otherwise be walked as source.

Each repository can still run its own `sf check` for its own rules. The
workspace runs the ones that are about the relationship.

## Checks this tool cannot express

Some drift is only decidable by regenerating: export the API schema, run the
generator, compare. No glob or query says that, and a hash lock cannot either,
because the artifact is *supposed* to change whenever its source does.

`L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE` runs a command instead:

```yaml
  L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE:
    enabled: true
    options:
      run: "make export-openapi && git diff --exit-code -- contracts/"
```

What that buys over a plain CI step is everything around it: the written reason
printed where it fails, a mutation fixture proving it still fails when it
should, a place in the same report, and a policy that cannot be quietly
loosened. It is also the bridge for a bespoke checker a team already has —
wrap it, and it gets the same treatment as everything else.

Commands are **refused unless asked for**:

```sh
sf check --allow-commands      # or SF_ALLOW_COMMANDS=1
```

A policy file is data that travels with a clone, so running whatever it says
would make cloning a repository dangerous. Without the flag the rule still
reports — a check that did not run is not a check that passed.
