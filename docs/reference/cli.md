# CLI reference

Every command, flag and default on this page is read out of the command-line
definition in `src/main.rs` on every build. If it is here, this binary accepts
it.

```sh
sf [--root <PATH>] <command> [options]
```

## Global options

<!-- sf:generated cli-global -->
| Option | What it does |
| :-- | :-- |
| `--root <ROOT>` | Repository to operate on. |

<!-- sf:end cli-global -->

`--help` and `--version` are accepted everywhere and are left out of the
tables below.

## Every command

<!-- sf:generated cli-index -->
| Command | What it does |
| :-- | :-- |
| [`sf catalog`](#sf-catalog) | List the catalog |
| [`sf check`](#sf-check) | Run every enabled rule |
| [`sf docs`](#sf-docs) | Regenerate documentation. Add --check to make this read-only |
| [`sf explain`](#sf-explain) | Print a rule: what it requires, why it exists, how to fix a violation |
| [`sf fixtures`](#sf-fixtures) | Write the mutation fixtures for every enabled rule |
| [`sf init`](#sf-init) | Scaffold policy, docs, CI, hooks and mutation fixtures into a repository |
| [`sf interview`](#sf-interview) | Print the decision tree an interview walks, and what each answer does |
| [`sf lock`](#sf-lock) | Rewrite the hash locks from what is on disk |
| [`sf ratchet`](#sf-ratchet) | Freeze today's violations so a repository can adopt rules it breaks |
| [`sf seal`](#sf-seal) | Recompute the digests in a gate's evidence manifest |
| [`sf skills`](#sf-skills) | Install the agent skills that drive this tool |
| [`sf verify`](#sf-verify) | Prove every enabled rule fires on its mutation fixture |

<!-- sf:end cli-index -->

## What each one does

<!-- sf:generated cli-detail -->
### `sf catalog`

List the catalog

```sh
sf catalog [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--layer <LAYER>` | List one layer only, by its identifier: L0 through L6 | none |

### `sf check`

Run every enabled rule

```sh
sf check [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--format <FORMAT>` | How to print the report: text for a person, json for a machine, markdown for a pull request comment | `text` |
| `--changed <CHANGED>` | Git ref to diff against, so gates activate from touched paths and the policy can be compared with the one being replaced | none |
| `--rule <RULE>` | Run one rule only | none |
| `--allow-commands` | Let `command` rules actually run. | off |
| `--policy <POLICY>` | Govern this run with a policy that lives somewhere else: a directory carrying `policy.yaml` and optional `rules/`, read read-only over a repository carrying none of its own. | none |

### `sf docs`

Regenerate documentation. Add --check to make this read-only

```sh
sf docs [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--check` | Write nothing. | off |

### `sf explain`

Print a rule: what it requires, why it exists, how to fix a violation

```sh
sf explain <RULE>
```

No flags of its own; the global options above apply.

### `sf fixtures`

Write the mutation fixtures for every enabled rule

```sh
sf fixtures
```

No flags of its own; the global options above apply.

### `sf init`

Scaffold policy, docs, CI, hooks and mutation fixtures into a repository

```sh
sf init [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--name <NAME>` | Project name recorded in the policy | none |
| `--language <LANGUAGE>` | Languages to parse: python, typescript, go, rust, ruby | `python`, `typescript`, `go` |
| `--layer <LAYER>` | Layers to enable. | `L1`, `L4`, `L5` |
| `--force` | Overwrite an existing policy | off |
| `--rules-document <RULES_DOCUMENT>` | Where to write the rule reference. | none |
| `--answers <ANSWERS>` | Answers from a `factory-init` interview. | none |

### `sf interview`

Print the decision tree an interview walks, and what each answer does

```sh
sf interview [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--json` | Machine-readable, for an agent conducting the interview | off |

### `sf lock`

Rewrite the hash locks from what is on disk

```sh
sf lock
```

No flags of its own; the global options above apply.

### `sf ratchet`

Freeze today's violations so a repository can adopt rules it breaks

```sh
sf ratchet [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--months <MONTHS>` | Months until the frozen entries must be reviewed | `6` |

### `sf seal`

Recompute the digests in a gate's evidence manifest

```sh
sf seal <GATE>
```

No flags of its own; the global options above apply.

### `sf skills`

Install the agent skills that drive this tool

```sh
sf skills [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--dir <DIR>` | Where to write them. | none |
| `--project` | This repository only: `<root>/.claude/skills` | off |
| `--user` | Every project on this machine: `~/.claude/skills` | off |

### `sf verify`

Prove every enabled rule fires on its mutation fixture

```sh
sf verify [options]
```

| Flag | What it does | Default |
| :-- | :-- | :-- |
| `--rule <RULE>` | Prove one rule only | none |
| `--allow-commands` | Let `command` rules actually run, so a command rule can be proven to fire rather than reported as unproven | off |
<!-- sf:end cli-detail -->

## Exit codes

Exit codes are hierarchical, so a caller can tell "the tool could not run" from
"the repository has violations": `3` bootstrap failed, `2` config error, `1`
findings, `0` clean.
