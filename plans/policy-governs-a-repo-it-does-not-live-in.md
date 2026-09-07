# A policy can govern a repository it does not live in

Today a policy applies to a repository by sitting inside it. There is no
supported way to point this tool at code you are not allowed, or not yet
ready, to commit `.software-factory/` into: somebody else's service, a
package in a monorepo whose owners have not agreed to the directory, a
contractor's checkout, or your own repository on the afternoon you are
deciding whether any of this is worth proposing.

## What the tool actually does now

The four paths are constants, all relative to one root:
`.software-factory/policy.yaml`, `.software-factory/ratchet.yaml`,
`.software-factory/rules` and `.software-factory/mutations`
([`src/policy.rs`](../src/policy.rs)). Discovery walks up from the working
directory until it finds a policy or a `.git`. The global `--root` chooses
*which* repository is read, never where its policy comes from.

One detail is worth stating because it changes what the complaint is. The lock
check reads digests of files on disk and never asks git anything, so dropping
an uncommitted `.software-factory/` into a clone and running `sf check` works
today. What does not work is everything after that single local run: CI has
nothing to check out, the ratchet has nowhere to live, the gates name plans
that are not in that repository, and the next person gets none of it. So the
honest form of the problem is not "the tool refuses", it is "there is no way
to apply a policy from outside that anybody can rely on twice".

## What this adds

A read path, and nothing else:

```sh
sf check --policy ../factory-policy/.software-factory
```

A directory rather than a file, because a policy that cannot carry its
`rules/` is a policy that cannot carry the half of itself that is local. The
run is read-only, and the report names the policy that produced it.

## Why this is not a hole in the guardrail

The vendored form's argument is undeniability, not location:
`L2.FACTORY_CONFIG_IS_LOCKED` makes a guardrail edit a second deliberate line
in a diff a code owner watches. An overlay moves that diff into another
repository, which keeps the property as long as the overlay directory is
itself under these rules. It is the same trade the tool already accepts for
`kind: command` rules: the reviewable thing has to be somewhere, and saying
where is the whole job.

What would be a hole is an overlay used to answer a red build. A repository
that carries its own policy and is then checked against a different one from
outside has two answers to "what governs this", and the second one is
whichever answer somebody preferred. So `--policy` is refused when the root
already carries a policy of its own, and every subcommand that writes
repo-local state refuses the flag outright: `sf init`, `sf ratchet`,
`sf lock`, `sf fixtures` and `sf seal`. A ratchet written from a policy that
is not in the repository is debt nobody can find again.

## What an overlay gives up, out loud

- **The frozen baseline.** No ratchet, so an overlay run on a legacy
  repository reports everything it finds. That is the right answer for a
  report and the wrong one for a build gate.
- **Gates.** L3 reads `plans/` and an evidence manifest inside the repository
  being checked. Under an overlay those rules are inapplicable, and they have
  to say so per rule rather than pass, which is the argument
  `L5.NO_INERT_RULE` already makes about a rule that cannot produce a finding.
- **Evidence.** An overlay run is not proof of anything, so it cannot be
  sealed.
- **Its own protection over the target.** There is nothing in the target to
  lock or to compare directions on. Both L2 rules apply to the overlay in the
  repository where it lives.

Commands stay refused by default. The README's reason, that a policy is data
travelling with a clone, does not weaken here, it changes owner: the policy is
now yours and the code is the stranger. A `kind: command` rule still runs the
stranger's build, so the flag stays exactly as it is.

## Deliberately not in scope

Fetching a policy at check time. An overlay is a local path, for the same
reason [rule packs](third-party-rule-packs.md) vendor instead of fetching: a
rule is executable data, and `sf check` does not acquire executable data while
running. An overlay is also not an install path for a pack. A pack still
vendors into the repository that adopts it, and a repository that wants
adoption wants the directory.

## Acceptance criteria

- [ ] `sf check --policy <dir>` runs every rule that policy enables against a
      root carrying no `.software-factory/`, and reports what the same policy
      reports over the same code when vendored
      (proof: test:src/policy.rs)
- [ ] `--policy` is refused when the root carries a policy of its own, so no
      run has two answers to what governs the repository
      (proof: test:src/policy.rs)
- [ ] `sf init`, `sf ratchet`, `sf lock`, `sf fixtures` and `sf seal` refuse
      the flag rather than writing repo-local state from a policy that is not
      in the repository
      (proof: test:src/main.rs)
- [ ] Every report format names the overlay's path and digest and states that
      the run carried no ratchet, so a green overlay run cannot be quoted as a
      governed one
      (proof: test:src/report.rs)
- [ ] A rule needing repo-local state the overlay cannot supply reports as
      inapplicable with the reason, never as a rule that found nothing
      (proof: test:src/checks/mod.rs)
- [ ] `sf verify` run against the overlay directory proves the overlay's own
      rules fire, so governing from outside cannot ship rules nothing trips
      (proof: test:src/verify.rs)
- [ ] Where the frozen baseline lives for a repository governed from outside
      is decided before the flag ships
      (proof: unspecified:whether an overlay carries a ratchet keyed by target
      repository is a question about who owns the debt, and no check can
      answer it)
- [ ] A command rule under an overlay names which policy asked for it in the
      report
      (proof: deferred:the command path records no policy origin yet)

**Exit condition:** a repository with no `.software-factory/` of its own is
checked against a policy living in another repository, `sf check --policy`
reports over that code what the same policy reports when vendored into it, the
flag is refused against a root that carries its own policy and by every
subcommand that writes, and each report format names the policy that produced
the run.
