# Documentation

The rule this documentation is built to is one sentence:

> **Anything a machine can read out of the code is never written by hand.**

Every table of rules, commands, flags, languages, hazard tools, templates,
skills and interview decisions on these pages was read out of the thing it
describes. The prose between them is written by people. The build goes red when
the two disagree.

## The two commands

```sh
sf docs           # rewrite everything derived from the code
sf docs --check   # write nothing; exit non-zero if anything would change
```

`sf docs --check` is what `L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE` runs in
this repository, so a change to the catalog, the policy, the ratchet or the
command surface fails the build until the pages follow:

```
documentation is out of date:
  README.md
  docs/reference/cli.md

  run `sf docs` and commit the result
```

The check writes nothing, and that is load-bearing rather than tidy. The first
version of this rule ran `sf docs` and diffed the result, which meant a
read-only run rewrote the file it was checking. `L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE@release`
makes the same argument about `sf lock`, after a draft of it erased the evidence
the lock rules exist to read.

## What is derived, and from where

| Fact | Read from |
| :-- | :-- |
| Every rule's layer, statement, why, fix and defaults | `catalog/` |
| How many rules ship, are enabled, are off, carry a fixture, are frozen | The catalog, `policy.yaml`, `ratchet.yaml` |
| Rules per layer, and how many are on here | The catalog and the policy |
| Every command, its flags and their defaults | The clap definition in `src/main.rs` |
| Which rules carry a query for which language | The catalog's per-language queries |
| Which tool covers which hazard in which ecosystem | The `tools:` map on each L6 toolchain rule |
| The templates, the rule each writes, and its placeholders | `templates/` |
| The skills and what each is for | The front matter of each `skills/*/SKILL.md` |
| Every interview decision, answer and effect | `interview/decisions.yaml` |

Nothing generated carries a timestamp, a duration or a run count. A generated
file that changes every time it is generated cannot be checked for drift.

## How a page places a block

A page is markdown with markers in it:

```markdown
## This repository checks itself

<!-- sf:generated rules-summary -->
<!-- sf:end rules-summary -->
```

Everything **outside** the markers belongs to whoever wrote it and is never
touched. Everything inside is replaced. A marker inside a fenced code block,
like the one above, is documentation showing the form and is left alone, which
is the same distinction `L4.CLAIM_CITES_ITS_EVIDENCE` draws about its own
marker.

Three things are errors rather than warnings:

- **A page naming a block nothing produces.** A reference section that quietly
  renders nothing is worse than a build that fails.
- **A block nothing places.** A fact the documentation has and does not show is
  the same failure as one that is out of date.
- **A marker with no end marker**, which would otherwise silently swallow the
  rest of the page.

## The rules document

`docs/rules.md` is the one page generated wholesale below its first `## L`
heading, out of the catalog and this repository's policy. Everything above that
heading is this repository's own reasoning about which rules it runs and why
four of them are off, and `sf docs` never touches it.

## Adding a generated block

1. Write the renderer in `src/docs.rs` and add its name to `BLOCKS`.
2. Put the markers on the page that should show it.
3. Run `sf docs`.

Step 2 is not optional: a block nothing places fails the generator, which is
what stops a fact being computed and then never shown.

## Writing the prose half

- **Say why, not what.** The generated tables already say what. A page that
  restates them is a page that will be skipped.
- **Name the failure.** Nearly every rule here exists because something went
  wrong. The sentence that names it is the one people remember.
- **Link, do not repeat.** One page says a thing and the rest point at it.
  Repetition is how documentation drifts even when nobody edits it.
