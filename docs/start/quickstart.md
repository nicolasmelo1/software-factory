# Quickstart

Works on any repository, in any language. Nothing to configure first, and it
will not change a line of your code.

## 1. Install — 1 min

Download the binary for your platform from the
[latest release](https://github.com/nicolasmelo1/software-factory/releases/latest)
— no toolchain, no compile:

```sh
# macOS on Apple silicon; swap the target for x86_64-apple-darwin
# or x86_64-unknown-linux-gnu
curl -fsSLO https://github.com/nicolasmelo1/software-factory/releases/latest/download/sf-aarch64-apple-darwin
curl -fsSLO https://github.com/nicolasmelo1/software-factory/releases/latest/download/sf-aarch64-apple-darwin.sha256
shasum -a 256 -c sf-aarch64-apple-darwin.sha256
chmod +x sf-aarch64-apple-darwin && mv sf-aarch64-apple-darwin ~/.local/bin/sf
```

Or build it, if you have cargo:

```sh
cargo install --git https://github.com/nicolasmelo1/software-factory --tag v0.4.0 --locked
```

**Pin the tag.** The rule catalog ships *inside* the binary, so tracking the
tip of `main` means an upstream commit can change what an enabled rule matches
and turn your build red with nothing in your repository having moved. `sf init`
writes the same pinned form into the CI workflow it generates.

`sf --version` reports the version *and* the catalog digest, because the
version number alone does not identify the rules:

```
sf 0.4.0 (catalog f4c2b4b783a5, 38 rules)
```

One static binary, no runtime, nothing to clone. Building it is a single
compile — around a minute cold, a few seconds if you already have the crates
cached. Everything below runs in well under a second, even on a large monorepo.

**If `sf` is then "command not found":** cargo installed it to `~/.cargo/bin`,
which is not on your `PATH`. The `rustup` installer adds that directory for
you; Homebrew, `apt` and Nix do not, and cargo says so in a warning at the end
of the install that is easy to scroll past. Fix it once:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && exec zsh
#                                               ~/.bashrc for bash
```

## 2. Set it up — 2 min

```sh
cd ~/code/your-project
sf skills
```

It asks where. There is no default on purpose — these skills are about *this*
repository's factory, and writing them into every project on the machine is a
decision you should make rather than one this makes for you:

```
Where should the skills go?

  1  /Users/you/code/your-project/.claude/skills   (this repository only)
  2  /Users/you/.claude/skills                     (every project on this machine)

[1]
```

Use `--project` or `--user` to skip the question, `--dir` for anywhere else. In
a script or CI it refuses to guess and tells you to pass one.

Then invoke the skill **by name**. It will not be reached for on its own:

```
/factory-init set up software-factory in this repo
```

The [`factory-init`](../../skills/factory-init/SKILL.md) skill takes it from there. It
reads your codebase first and answers whatever the code can answer itself —
which framework, where the client lives, whether there are already
`domain/`/`application/` directories. Then it asks you the rest in rounds, each
question numbered, each with a recommendation:

> ❓ **Q2** — **Architecture**: I can see `packages/*/domain/`, `application/`
> and `infrastructure/`, so this looks domain-driven. But
> `apps/api/src/routes/users.ts` opens a database connection directly. Is the
> layering the intent or the reality?
>
> ➡️ I'd answer `ddd` and freeze today's 40 violations with a six-month review
> date — but pick that deliberately, because it commits you to fixing them.

Layered or DDD, repositories or ORM-in-services, Zod or Pydantic and where
those schemas live, which packages the client must never import, whether
anything shares mutable state across threads. Those answers generate rules
carrying *your own* package names — not a generic starter set. See
[The interview](../start/the-interview.md) for the full decision tree.

<details>
<summary><b>No agent? One command instead.</b></summary>

```sh
cd ~/code/your-project
sf init --language typescript --layer L1,L4,L5,L6
git config core.hooksPath .githooks
```

Use `--language` for what you actually have: `python`, `typescript`, `go`,
`rust`, `ruby`, or several comma-separated. `--layer L1,L4,L5,L6` is the honest day-one
set — code quality, documentation cadence, the self-proving layer, and the
security tooling. You get the generic rules; the structural ones stay off until
you run the interview or write them yourself.

</details>

Either way, you end up with something like:

```
wrote 47 files:
  .software-factory/policy.yaml          # which rules are on
  docs/rules.md                          # why each one exists
  .allowed-root-files
  .github/workflows/software-factory.yml # CI, with the security tools wired in
  .githooks/pre-commit
  .software-factory/mutations/...        # a tiny broken repo per rule
  .software-factory/ratchet.yaml (106 existing violations frozen)
```

**Nothing is red yet.** Every violation that already existed was frozen with a
six-month review date. Only *new* ones fail — which is what makes this
adoptable on a codebase with years of history.

## 3. Prove the checks actually work — 10 sec

```sh
sf verify
```

```
✓ L1.NO_BLANKET_SUPPRESSION — 1 finding(s): Bare `# noqa` disables every rule on the line...
✓ L4.DOC_LINKS_RESOLVE — 1 finding(s): link target `../src/pricing/README.md` does not exist
...
17/17 enabled rules proven to fire
```

Every rule was just run against a repository built to violate it. This is the
step that separates enforcement from decoration: a check with a typo in it
passes silently forever and looks exactly like a check that works.

## 4. Watch it catch something — 30 sec

```sh
echo "# scratch notes" > NOTES.md
sf check
```

```
! medium L4.ROOT_FILES_ARE_DECLARED — New top-level files are declared before they appear
  why  `NOTES.md`, `PLAN.md`, `SUMMARY.md` at the repository root is the most
       recognizable signature of agent-authored work, and each one is context
       that belongs in a plan, a pull request body or a commit message —
       somewhere with a lifecycle.
  fix  Move the content to the plans directory or the pull request
       description. If the file really belongs at the root, add it to the
       allowlist in the same commit.
    NOTES.md — `NOTES.md` is at the repository root but not declared

1 findings across 1 rules (106 frozen by the ratchet)
```

```sh
rm NOTES.md   # green again
```

That output shape is the design. The failure message is the only documentation
an agent reliably reads, so every rule carries its reasoning to the point of
failure rather than leaving it in a file nobody opens.

## 5. Turn it on for real — 1 min

The generated workflow already runs `sf verify` then `sf check` on every pull
request, with the security tooling for your languages wired in. Commit it:

```sh
git add -A && git commit -m "chore: adopt software-factory"
```

Exit codes are hierarchical, so CI can tell the difference between "the tool
could not run" and "the repository has violations": `3` bootstrap failed,
`2` config error, `1` findings, `0` clean.

## If something goes wrong

| | |
|---|---|
| `sf check` red on `L5.NO_INERT_RULE` | A rule is switched on and pointed at nothing here. Give it a scope, or disable it in `policy.yaml` and write down why. |
| Too many findings to face | `sf ratchet --months 6` freezes today's state. It is debt with a due date, not permission. |
| A rule seems wrong | `sf explain <RULE>` gives the full reasoning. If it is genuinely wrong, change it — that is one of the four honest resolutions. |
| Want to see everything available | `sf catalog`, and `sf interview` for the decisions that generate rules. |
| `sf: command not found` after installing | `~/.cargo/bin` is not on your `PATH` — see step 1. |

---
