# software factory

**A method for building software with agents, packaged as a single binary that
runs against any repository.**

> **Every rule that matters is written twice, once as prose that says *why*,
> once as a check that *fails*. And every check has a mutation that proves it
> fires.**

Generating code stopped being the hard part. Generating the *right* code, code
that fits the requirement, respects the boundary, and can be trusted without a
senior engineer reconstructing the whole chain of intent behind it, is still
the bottleneck. Teams solve it by hand-rolling an internal harness of prompts,
conventions, review rituals and glue scripts, and then discover the harness has
become a second codebase nobody budgeted for.

`sf` is that harness, extracted from a working one, made language-neutral, and
reduced to the sentence above. It protects its own configuration, so an agent
cannot reach a green build by turning a rule off.

---

## Install

```sh
cargo install --git https://github.com/nicolasmelo1/software-factory --tag v0.4.0 --locked
```

Or download a binary for your platform from the
[latest release](https://github.com/nicolasmelo1/software-factory/releases/latest):
no toolchain, no compile. Both forms are spelled out in
[the quickstart](docs/start/quickstart.md).

**Pin the tag.** The rule catalog ships *inside* the binary, so tracking the
tip of `main` means an upstream commit can change what an enabled rule matches
and turn your build red with nothing in your repository having moved.

`sf --version` reports the version *and* the catalog digest, because the
version number alone does not identify the rules:

```
sf 0.4.0 (catalog f4c2b4b783a5, 38 rules)
```

---

## Sixty seconds

```sh
cd ~/code/your-project
sf skills                 # install the agent skills, then run /factory-init
```

Or without an agent:

```sh
sf init --language typescript --layer L1,L4,L5,L6
sf verify && sf check
```

Nothing is red on the first run. Every violation that already existed is frozen
with a review date, and only new ones fail, which is what makes this adoptable
on a codebase with years of history. The five-minute version, with the output
at each step, is [the quickstart](docs/start/quickstart.md).

---

## Documentation

Everything a machine can read out of this repository is generated into these
pages, and `sf docs --check` fails the build when the code moves and a page
does not. How that works is [written down](docs/development/documentation.md).

**Start here**

| | |
| :-- | :-- |
| [Quickstart](docs/start/quickstart.md) | Five minutes from nothing to a build that blocks the next bad change |
| [Adopting a repository with history](docs/start/adopting-a-repo-with-history.md) | Which layers first, and how to freeze what is already broken |
| [The interview](docs/start/the-interview.md) | The decisions that generate rules carrying your own names |
| [The skills](docs/start/the-skills.md) | The four agent skills, and the boundary all of them keep |

**How it works**

| | |
| :-- | :-- |
| [The method](docs/method.md) | The reasoning behind the layering |
| [The seven layers](docs/concepts/the-seven-layers.md) | What each layer is about, and how much of it is on here |
| [Layer 5 is the whole point](docs/concepts/layer-5-is-the-point.md) | Why every check ships with a repository built to break it |
| [The gate](docs/concepts/the-gate.md) | L3 in practice: evidence that expires when the code moves |
| [Stopping an agent from relaxing the rules](docs/concepts/locking-the-guardrail.md) | The two rules that make a weakened guardrail undeniable |
| [Hunting defect classes](docs/concepts/hunting-defect-classes.md) | L6, and what static analysis honestly cannot do |

**Reference**

| | |
| :-- | :-- |
| [CLI](docs/reference/cli.md) | Every command and flag, read out of the binary |
| [Rules](docs/rules.md) | Every rule this repository enforces, and why |
| [What a rule looks like](docs/reference/writing-a-rule.md) | The shape of a catalog entry, and the three kinds of check |
| [Pointing the policy at your repository](docs/reference/policy.md) | Per-package settings, several checkouts, command checks |
| [Language coverage](docs/reference/languages.md) | Which rules reach which languages |
| [Rule templates](docs/reference/templates.md) | The rules an interview fills in with your own names |

---

## What this is not

It does not replace your linter, type checker or test suite. It enforces the
decisions those tools have no opinion about. It does not review code for
correctness. It does not run your tests; L3 checks that *something* ran, proved
what it claimed, and has not gone stale since.

And it is deliberately small. A method you cannot read in an afternoon is a
method nobody will adopt.

---

## This repository checks itself

`sf` is written in Rust and Rust is one of its target languages, so this
repository runs its own rules against its own source, with its own mutation
fixtures, in its own CI.

<!-- sf:generated rules-summary -->
**39 rules shipped**, 35 enabled here, 4 switched off, 35 carrying a mutation fixture, 3 violations frozen.

Frozen, with a date the build fails on: `L4.PLAN_PROOF_BUDGET` by 2027-03-04, `L6.PERFORMANCE_REGRESSION_IS_GUARDED` by 2027-02-18.
<!-- sf:end rules-summary -->

Every rule switched off here is off as a written decision in
[`docs/rules.md`](docs/rules.md), because `L5.NO_INERT_RULE` refuses to let a
rule be enabled and pointed at nothing.

## Contributors

[![Contributors](https://contrib.rocks/image?repo=nicolasmelo1/software-factory)](https://github.com/nicolasmelo1/software-factory/graphs/contributors)

## License

MIT.
