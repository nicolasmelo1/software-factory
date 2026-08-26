# Close the inert-rule blind spot for TextPattern and Complexity

`L5.NO_INERT_RULE` (`inert_rules()` in `src/checks/cadence.rs`) checked five of
the eleven check kinds: an enabled lock with an empty scope, a command rule
with no `run`, a toolchain rule with no tools, and a shape or nested rule with
no query for a declared language. It never looked at `TextPattern` or
`Complexity`, which are exactly the kinds L1 is built from.

This was not theoretical. Measured on `~/Sites/pp-team/postpilot` (365,133
lines of Ruby, 4,683 `.rb` files, zero `.ts .tsx .jsx .py .go .rs`) with
`languages: [typescript]` declared: `L1.COMPLEXITY_CEILING`,
`L1.NO_BLANKET_SUPPRESSION` and `L1.NO_UNTYPED_ESCAPE_HATCH` all reported
nothing across the whole repository, and `L5.NO_INERT_RULE` declared the
policy sound. A rule that passes every run forever and shows up in every
report as "found nothing" is indistinguishable from a rule that is protecting
you — which is the precise failure this rule exists to catch, and it was
blind to its own two most common kinds.

## The decision that mattered: what "inert" means for a text-pattern rule

Two candidate tests were considered for `TextPattern`:

1. **The scope glob matches zero files in the repository.**
2. **The scope names no extension belonging to a language the project
   declares.**

They differ, and the difference is the whole point. `L1.NO_BLANKET_SUPPRESSION`
and `L1.NO_UNTYPED_ESCAPE_HATCH` both default their scope to
`**/*.py, **/*.ts, **/*.tsx, **/*.go, **/*.rs` — a fixed set of extensions,
independent of what the repository's policy declares. Test 2 would read that
scope as "covers typescript" the moment a repository declares
`languages: [typescript]`, and say the rule is fine — exactly the state
postpilot was in, and exactly the state that was silently wrong. Test 1 asks
the only question that is actually true or false in this repository: did the
glob select a single file. Chosen: **test 1**, implemented as
`scan::select(ctx.files, &options.scope, &options.exclude)?.is_empty()` — the
same selection the check itself runs, so the inertness test can never disagree
with the check about what it can see.

## Complexity

`checks::complexity::run` silently skips a file for two independent reasons:
`Lang::from_path` cannot classify the extension, or the classified language is
not in `ctx.policy.project.languages`. Either one alone leaves the rule
inert if it holds for every file the scope selects. `any_parseable_declared_file`
in `cadence.rs` mirrors both skip conditions exactly and asks whether any
selected file survives both — the same combined question `complexity::run`
answers per file, asked once in aggregate.

## False-positive risk

The failure mode this change most easily produces is flagging a rule that is
genuinely fine — silently loosening a check to "fix" a false alarm is exactly
what `L2.POLICY_ONLY_TIGHTENS` and this repository's own method exist to
catch, so the test had to be checked against a codebase where the affected
rules are real. `software-factory` itself declares `languages: [rust]` and has
its own `.rs` files under the default text-pattern scopes, so
`L1.NO_BLANKET_SUPPRESSION`, `L1.NO_UNTYPED_ESCAPE_HATCH` and
`L1.COMPLEXITY_CEILING` all select real files here — `sf check` on this
repository stays clean under the new detection.

## Evidence

- `sf verify` fires `L5.NO_INERT_RULE` on the extended fixture
  (`.software-factory/mutations/L5.NO_INERT_RULE/`), which now enables one
  inert instance of each of the three kinds: `L2.GENERATED_FILES_ARE_LOCKED`
  (empty scope, the pre-existing case), `L1.NO_BLANKET_SUPPRESSION@inert` and
  `L1.COMPLEXITY_CEILING@inert` (both scoped at `nonexistent/**`, the new
  cases) — three separate findings under one rule id.
- `sf check` on `software-factory` itself stays clean.
- A temporary policy pointed at a read-only symlink to `~/Sites/pp-team/postpilot`
  with `languages: [typescript]` and the L1 layer enabled reproduces the
  original blindness on the binary built before this change (only the
  pre-existing `L1.SKIPPED_TESTS_STATE_A_REASON` finding — the Shape/Nested
  case already covered) and shows the binary built after this change naming
  all three previously-silent rules by id and reason.

## Non-goals

This does not touch `Lang::from_path`, `Lang::from_name`, any tree-sitter
grammar, or the `L6.*` toolchain lists — a repository declaring a language
`sf` cannot parse at all is a different, already-tracked gap
(`plans/expand-language-adapters.md`). This plan is only about the guardrail
noticing that gap in its own report, not about closing it.

**Exit condition:** `sf verify` shows `L5.NO_INERT_RULE` firing on a fixture
that is inert only through the `TextPattern` or `Complexity` path — proven —
and running the built `sf` against a real repository whose declared language
matches none of its files names the blind `L1.*` rules instead of staying
silent, proven by the postpilot reproduction above.
