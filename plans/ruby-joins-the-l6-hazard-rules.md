# Ruby joins the L6 hazard rules

`Lang::from_path` cannot classify `.rb`, so no L0 structural rule and no
mutation-fixture query will ever say anything about a Ruby repository — that
is real grammar work, tracked separately in
[Expand the language adapters](expand-language-adapters.md). The L6 hazard
layer is different: `src/checks/toolchain.rs` matches a tool by looking for
its raw name in a CI workflow, a Makefile or a task runner, keyed off the
policy's `languages:` list — no parsed syntax involved. That makes four of the
nine L6 rules meaningful for Ruby today, with zero grammar work, simply by
naming the tools a Ruby team actually runs.

## What changed

- `catalog/L6/dependency-vulnerabilities-are-scanned.yaml`,
  `insecure-patterns-are-scanned.yaml`, `secrets-are-scanned.yaml` and
  `workflows-are-scanned.yaml` each gained a `ruby:` tools list —
  `bundler-audit`/`bundle-audit`/`osv-scanner`, `brakeman`/`semgrep`/`codeql`,
  and the same language-neutral secret and workflow scanners the other four
  languages already declare.
- `sf init --language ruby --layer L6` now emits real CI steps for
  `bundler-audit` and `brakeman` (`gitleaks` and `zizmor` were already
  language-neutral steps emitted whenever any L6 rule is selected).
- `FIXTURES_HINT` names the Ruby exclude form for `rubocop`/`standardrb` and
  `rspec`, alongside `brakeman`'s own `--skip-files`, so a Rails team adopting
  `sf` knows to keep `.software-factory/mutations` out of its own tooling.
- `dependency_manifests()` already listed `Gemfile` before this change —
  verified, not touched.

## What deliberately did not change

- `L6.DEAD_CODE_IS_DETECTED` and `L6.PERFORMANCE_REGRESSION_IS_GUARDED` do
  **not** gain a `ruby:` key. There is no dead-code detector or benchmark
  harness for Ruby worth naming honestly here; a rule enabled with a tool that
  does not exist is exactly the lie `L5.NO_INERT_RULE` exists to catch. If a
  real Ruby tool for either concern shows up later, add it then — this is a
  decision, not an oversight.
- `L6.DATA_RACES_ARE_DETECTED` also stays without a `ruby:` key. MRI's GVL
  makes the concept marginal for Ruby, the same reasoning the rule already
  applies to omit Python and TypeScript.
- No grammar, no tree-sitter query, no `queries:` block, no change to
  `src/checks/cadence.rs`. Ruby structural coverage (L0–L1, L5's per-language
  mutation proof) is a separate, larger effort tracked in
  [Expand the language adapters](expand-language-adapters.md).

## Acceptance criteria

- [ ] Exactly the four rules with a real Ruby scanner —
      `L6.DEPENDENCY_VULNERABILITIES_ARE_SCANNED`,
      `L6.INSECURE_PATTERNS_ARE_SCANNED`, `L6.SECRETS_ARE_SCANNED` and
      `L6.WORKFLOWS_ARE_SCANNED` — declare a `ruby:` tools list, and no other
      L6 rule does.
      (proof: assertion:catalog.l6-ruby-tools-exactly-four)
- [ ] `sf init --language ruby --layer L1,L4,L5,L6` against an empty Ruby
      repository emits `bundler-audit`, `brakeman`, `gitleaks` and `zizmor`
      steps in the generated workflow, instead of falling through to
      `hazard_steps`'s empty arm.
      (proof: assertion:init.ruby-hazard-steps-emitted)
- [ ] Every enabled rule in this repository's own policy still fires on its
      mutation fixture — this change adds no fixture obligation, because
      `CheckKind::Toolchain` is not language-scoped the way
      `CheckKind::Shape`/`Nested` are, so the existing `CI_WITHOUT_HAZARD_TOOLS`
      fixture continues to prove all four rules regardless of the `ruby:`
      addition.
      (proof: assertion:verify.all-enabled-rules-fire)
- [ ] Pointed at a real Ruby repository whose CI runs `bundler-audit` but no
      `brakeman`, `gitleaks` or `zizmor`, `sf check` with `languages: [ruby]`
      and L6 enabled reports findings for the three missing scanners and
      nothing for the one present.
      (proof: assertion:check.postpilot-ruby-hazard-findings)

**Exit condition:** a Ruby repository's own CI workflow can be graded by `sf
check` for four of the nine L6 hazard concerns without any tree-sitter grammar
existing for Ruby — proven by pointing this repository's own binary at a real
Rails application's CI configuration and getting findings a maintainer of that
application would agree are real.
