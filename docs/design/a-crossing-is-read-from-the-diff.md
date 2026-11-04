---
base: 8004eddeef34a460abc21e41923ba928e6088c77
sources:
  - { id: S1, kind: review, said: "seven Copilot findings across five pull requests reached a mechanism their plan never answered: #73 left src/checks/mod.rs out of touches, so overlay went unanswered; #72, #67 and #68 likewise" }
  - { id: S2, kind: code, ref: "src/checks/plan_declaration.rs", symbol: [crossings, uncrossed, touched_paths], said: "the mechanisms a plan answers are derived only from its declared touches, never from what the work changed, so a file changed outside touches escapes crossings and scope" }
  - { id: S3, kind: code, ref: "src/main.rs", symbol: [changed_paths], said: "`sf check --changed <ref>` reads `git diff --name-only <ref>` into the changed set; without the flag nothing is read" }
  - { id: S4, kind: decision, said: "deterministic, and red before the push: a pre-commit hook runs `sf check --allow-commands` with no --changed, and CI runs it with --changed origin/$BASE_REF" }
  - { id: S5, kind: decision, said: "the changed set is --changed when given; else, under project.default_branch, the tree against `git merge-base HEAD <branch>`, read with -z; with neither the rule adds nothing" }
  - { id: S6, kind: decision, said: "a git failure while reading the changed set is reported, never read as an empty set" }
  - { id: S7, kind: decision, said: "a plan is in flight when its own file is in the changed set, in plans/ or delivered into the design notes in the same change; only in-flight plans meet the diff" }
  - { id: S8, kind: decision, said: "each changed file is covered by an in-flight plan's touches under the touches matcher, by the rule's incidental globs, or by a literal mention in deviations; otherwise untouched:<path>" }
  - { id: S9, kind: decision, said: "incidental defaults are safe anywhere: .software-factory/**, the plans board and the in-flight plan files; an incidental file reaches no mechanism" }
  - { id: S10, kind: decision, said: "mechanisms reached by changed files join those reached by touches, so a changed file reaching an unanswered mechanism is uncrossed:<id>, declared or deviated alike" }
  - { id: S11, kind: decision, said: "everything sorted and deduplicated: one tree gives the same findings in the same order" }
  - { id: S12, kind: decision, said: "widening incidental is a loosening `L2.POLICY_ONLY_TIGHTENS` reports" }
  - { id: S13, kind: decision, said: "it has to work here and in amy, which declares plans and its own docs/mechanisms.yaml the same way" }
touches:
  - src/checks/plan_declaration.rs
  - src/checks/tightening.rs
  - src/policy.rs
  - src/init.rs
  - src/fixtures.rs
  - catalog/L4/plan-is-a-linked-declaration.yaml
  - docs/rules.md
  - README.md
criteria:
  - { id: A1, statement: "a changed file no in-flight plan's touches, no incidental glob and no deviation covers is `untouched:<path>`; covered by any of them it is not", proof: "property:src/checks/plan_declaration.rs#a_prefix_covers", sources: [S1, S2, S8], done: true }
  - { id: A2, statement: "a changed file reaching a mechanism no crossing answers is `uncrossed:<id>`, even when only a deviation names it; an incidental file reaches none", proof: "test:src/checks/plan_declaration.rs", sources: [S2, S9, S10], done: true }
  - { id: A3, statement: "only a plan whose own file is in the changed set, in the plans scope or delivered, is checked against the diff", proof: "test:src/checks/plan_declaration.rs", sources: [S7], done: true }
  - { id: A4, statement: "--changed wins; else default_branch's merge-base diff, proven in a real git repository; with neither the rule reports what it reported before", proof: "test:src/checks/plan_declaration.rs", sources: [S3, S4, S5, S13], done: true }
  - { id: A5, statement: "a git failure reading the changed set is a `changed-set` finding naming the command and its error", proof: "test:src/checks/plan_declaration.rs", sources: [S6], done: true }
  - { id: A6, statement: "one tree's diff findings come out sorted and deduplicated, identical run to run", proof: "test:src/checks/plan_declaration.rs", sources: [S11], done: true }
  - { id: A7, statement: "`L2.POLICY_ONLY_TIGHTENS` reports a widened `incidental` and a dropped `default_branch`", proof: "test:src/checks/tightening.rs", sources: [S12], done: true }
  - { id: A8, statement: "the fixture trips `untouched` and `uncrossed` through the diff and its repair clears both", proof: "test:src/checks/plan_declaration.rs", sources: [S8, S10], done: true }
crosses:
  - { mechanism: plans, answer: works, proof: "test:src/checks/plan_declaration.rs (A1 to A6, A8)" }
  - { mechanism: policy-merge, answer: works, proof: "test:src/checks/tightening.rs (A7), where `incidental` merges one level over its default like every option" }
  - { mechanism: tightening, answer: works, proof: "test:src/checks/tightening.rs (A7)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/checks/plan_declaration.rs (A8); sf verify still proves the rule fires" }
  - { mechanism: catalog, answer: works, proof: "test:src/catalog.rs, which loads the rule under its unchanged id with its statement, fix and defaults grown" }
  - { mechanism: generated-docs, answer: works, proof: "test:src/docs.rs; rules.md is regenerated by sf docs and sf docs --check finds it in step" }
  - { mechanism: init-and-interview, answer: n/a, why: "`sf init` writes no default_branch, so a fresh policy reads no diff until somebody opts in" }
guarantee:
  - { holds: "a commit through the pre-commit hook or a pull request through CI cannot change a file no in-flight plan accounts for, or reach a mechanism it never answered", not_against: "a change carrying no in-flight plan, untracked files git diff does not list, a stale local default branch, or a deviation that names a path without saying anything true" }
failures:
  - { step: "reading the changed set", when: "git cannot resolve the merge-base or run the diff", goes_to: "a `changed-set` finding naming the command and git's error; never an empty set" }
  - { step: "reading the changed set", when: "neither --changed nor default_branch is given", goes_to: "the rule adds nothing, exactly as before" }
cut:
  - "checking plans not in flight against the diff: refused, a stamped plan's base drifts behind main and would fire on everybody's work (S7)"
  - "a separate rule id: the linked declaration still names what this checks (S2)"
deviations:
  - "A1 cites the property `a_prefix_covers` rather than `test:`; the plan said `test:` only so its own commit stayed green before the falsifier existed"
  - "a delivered plan is found through a new `delivered` option, default `docs/design/*.md`: the rule's scope reads only plans/, and S7 needs a delivered note read while in flight"
  - "every changed file in the plan scope is incidental, not only in-flight ones: a plan the change removed or delivered is not work to declare"
  - "a changed file is the crossing of the in-flight plan that declares or names it; a file nobody accounts for is every in-flight plan's"
  - "`L2.POLICY_ONLY_TIGHTENS` also reports a narrowed `delivered`, the same loosening as a widened `incidental`; its property now generates `incidental` too"
  - "git runs without GIT_DIR, GIT_WORK_TREE and GIT_INDEX_FILE, so a run inside a hook reads the repository at the root and nothing else"
  - "A8: `sf verify` runs a fixture with no changed set, so it proves the rule fires on the declaration refusals; the diff refusals trip and clear on the same fixture in the unit test"
  - "adding `default_branch` to `Project` moved src/init.rs, an adoption activation path: the gate was re-run against hono at 57ba76a and re-sealed"
  - "README.md is in touches and did not change: sf docs regenerated nothing there"
  - "`uncrossed` says whether the reach came from `touches` or from the diff"
  - "the row left the execution order on delivery, so plans/next-steps.md and the row numbers cited in a-seal-refuses-a-stale-report.md are back as they were"
exit: "in this repository and in amy, a change editing a file its plan never declared, or reaching a mechanism through one, goes red at the pre-commit hook before the push, and CI gives the same findings"
---
# A crossing is read from the diff

Delivered. This document was `plans/a-crossing-is-read-from-the-diff.md`, the plan this repository executed for it; it now lives here as the design note, with every criterion checked and what the work found under `deviations:`.

Seven review findings were one finding: the work reached a mechanism its plan never answered, because the crossings a plan owes were read from what it declared rather than from what it changed.
