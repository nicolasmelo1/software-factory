# Documentation

Half of what is here is generated. Every table of rules, commands, flags,
languages, hazard tools, templates, skills and interview decisions was read out
of the thing it describes, and `sf docs --check` fails the build when the two
disagree. The prose between those tables is written by hand.

**Start here**

| | |
| :-- | :-- |
| [Quickstart](start/quickstart.md) | Five minutes from nothing to a build that blocks the next bad change |
| [Adopting a repository with history](start/adopting-a-repo-with-history.md) | Which layers first, and how to freeze what is already broken |
| [The interview](start/the-interview.md) | The decisions that generate rules carrying your own names |
| [The skills](start/the-skills.md) | The four agent skills, and the boundary all of them keep |

**How it works**

| | |
| :-- | :-- |
| [The method](method.md) | The reasoning behind the layering |
| [The seven layers](concepts/the-seven-layers.md) | What each layer is about, and how much of it is on here |
| [Layer 5 is the whole point](concepts/layer-5-is-the-point.md) | Why every check ships with a repository built to break it |
| [The gate](concepts/the-gate.md) | L3 in practice: evidence that expires when the code moves |
| [Stopping an agent from relaxing the rules](concepts/locking-the-guardrail.md) | The two rules that make a weakened guardrail undeniable |
| [Hunting defect classes](concepts/hunting-defect-classes.md) | L6, and what static analysis honestly cannot do |

**Reference**

| | |
| :-- | :-- |
| [CLI](reference/cli.md) | Every command and flag, read out of the binary |
| [Rules](rules.md) | Every rule this repository enforces, and why |
| [What a rule looks like](reference/writing-a-rule.md) | The shape of a catalog entry, and the three kinds of check |
| [Pointing the policy at your repository](reference/policy.md) | Per-package settings, several checkouts, command checks |
| [Language coverage](reference/languages.md) | Which rules reach which languages |
| [Rule templates](reference/templates.md) | The rules an interview fills in with your own names |

**Development**

| | |
| :-- | :-- |
| [Documentation](development/documentation.md) | What is generated, from where, and how a page places a block |
