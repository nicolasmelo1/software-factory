# Scaffolds are proven in micro-sandboxes

Every convention a rule checks is a convention something could have written.

This tool has only ever had the checking half. A rule says a router looks like
this, a service looks like that, a query lives here. An agent then writes that
shape from scratch, in full, every time, and the rule confirms it guessed
right. The shape was known before the agent started. Paying a model to
rediscover it is the most expensive way to obtain a thing nobody was in doubt
about.

Rails settled this in 2005. The reason it is worth revisiting is not the
generator; it is that a generator normally has no way to stay honest, and this
tool does.

## The half that already exists

`sf init` is described in its own help text as "Scaffold policy, docs, CI,
hooks and mutation fixtures into a repository". `sf fixtures` writes a mutation
fixture per enabled rule, and `sf verify` proves each rule fires on its own
fixture. In `automate-my-work` that mechanism carries 26 rules, all 26 proven,
and the gate refuses a rule that cannot produce a finding.

A mutation fixture is already a micro-sandbox. The one carrying a local rule
in `automate-my-work` is three files: a `policy.yaml`, the rule itself, and
one source file that violates it. Self-contained, minimal, and the single property
it exists to demonstrate is demonstrated by running the real tool against it.

So the machinery for "generate a small thing, then prove the thing has a
property" is built, load-bearing, and aimed at exactly one target: rules about
rules. `templates/` holds four templates today and all four are rule
templates.

## What generation adds, and why it is safe here

A scaffold definition lives beside the local rules that govern what it emits,
in `.software-factory/scaffolds/<name>/` next to `.software-factory/rules/`.
It stays in the repository whose conventions it encodes. This tool ships the
mechanism and never the shapes: a router in one codebase is not a router in
another, and a catalogue of somebody else's shapes is the thing nobody wants
installed.

`sf verify` then grows a second obligation, under a rule this plan adds. For
every scaffold:

1. what it emits **passes** the rules that govern those paths
2. a mutated copy of what it emits **fails** them

The second half is the one that matters, and it is the same argument
`L5.NO_INERT_RULE` makes: output that no rule can reject proves nothing about
the rules or the scaffold.

**This is what stops generator rot.** A generator is usually a liability
because the convention moves and the generator keeps emitting last year's
shape, and nobody finds out until review. Here the rule and the scaffold are
two statements of one convention, and the build fails the moment they
disagree. Whichever is wrong, the disagreement itself is the finding.

## The agent does not have to be told

The obvious way to make an agent use a scaffold is to list the scaffolds in
its prompt. That pays tokens for the list on every call, and the list rots.

The better way is the one already in use everywhere else in this tool: the
rule fires on hand-written code that does not match the shape, and the
finding's `fix` names the command that would have produced it. Discovery is a
gate finding, not a paragraph of instructions. An agent that has never heard
of scaffolds meets them the first time it writes a router by hand.

## The risk, stated plainly

A scaffold earns its maintenance only where the shape genuinely repeats.
Rails' generators are also the thing people fight when the shape is nearly
right, and a mandatory generator producing the wrong shape is worse than no
generator: the work stalls and the only way out is to argue with a tool.

So the shape a scaffold emits is a floor and never a ceiling. Declining a
scaffold has to remain possible, with the reason recorded, and a rule may
require the shape only where it is genuinely required. A scaffold nobody
chooses is a scaffold to delete, and the usage record is what says which one
that is.

## Acceptance criteria

- [ ] A scaffold definition in `.software-factory/scaffolds/` emits its files
      into a target repository, and emits nothing outside the paths it
      declares
      (proof: assertion:scaffold.writes_only_what_it_declares)
- [ ] `sf verify` refuses a scaffold whose output violates a rule that
      governs the paths it writes to, so a scaffold and a rule cannot drift
      apart silently
      (proof: test:src/scaffold.rs)
- [ ] `sf verify` refuses a scaffold whose output no rule can reject, on the
      same argument `L5.NO_INERT_RULE` makes about a rule nothing can trip
      (proof: test:src/scaffold.rs)
- [ ] A micro-sandbox generated for a scaffold author runs the real tool
      against generated output without a checkout of the host repository
      (proof: assertion:sandbox.proves_generated_output_standalone)
- [ ] A rule that governs a scaffolded path names the scaffold command in its
      `fix`, so an agent writing the shape by hand is told what would have
      written it
      (proof: test:src/checks/prose.rs)
- [ ] Whether declining a scaffold is recorded in the repository or left to
      the caller is decided before any check ships
      (proof: unspecified:a decision about where usage lives, which no check
      can validate, because the answer changes what the record is for)

**Exit condition:** a repository declares a scaffold, `sf scaffold <name>`
writes a router, a service and a query that `sf check` passes with no
hand-editing, `sf verify` goes red when that scaffold's output is changed to
violate a rule that governs it and red again when every rule over those paths
is removed, and an agent that writes one of those shapes by hand receives a
finding naming the command it should have run.
