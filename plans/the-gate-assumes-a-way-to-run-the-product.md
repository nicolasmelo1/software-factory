---
sources:
  - { id: S1, kind: reference, ref: "docs/method.md", said: "L3's five properties are of the report (re-hashed, assertions re-checked, digest expires, goal and actor read for leaked answers); all start after the product runs, the expensive step sf says nothing about" }
  - { id: S2, kind: code, ref: "src/init.rs", said: "`sf init` (`policy_document`) writes `gates: {}` into every scaffolded policy: an adopter enabling L3 gets an empty map and no first gate" }
  - { id: S3, kind: code, ref: "interview/decisions.yaml", said: "the thirteen decisions never mention L3 or a gate; the only route is hand-writing policy entry, manifest, report and producer at once" }
  - { id: S4, kind: code, ref: "skills/factory-evidence/SKILL.md", said: "step 3 is the whole surface for producing a run: 'Start the actual entry point a customer would use. Drive it the way they would.'" }
  - { id: S5, kind: code, ref: ".software-factory/evidence/adoption-scenario.sh", said: "the one worked example anywhere, 4.5 KB of shell; nothing generates or regenerates it, and no rule reads it" }
  - { id: S6, kind: reference, ref: "pstack create-verification-skill", said: "interviews the codebase, writes `.claude/skills/verify-<app>/SKILL.md` (launch, doctor, drive, evidence, cleanup) and `features/` with observable success criteria, runs the loop once; reportedly brought Clerk up locally" }
  - { id: S7, kind: decision, said: "discovery (a half hour for a multi-tenant stack) is spent once and kept in the repository; redoing it per run invites a polished report of a run that never happened" }
  - { id: S8, kind: decision, said: "a feature with an observable success criterion is a required assertion: pstack's feature map and our `required_assertions` are two spellings of one list" }
  - { id: S9, kind: decision, said: "a markdown artifact is wrong for us: unreproducible, unhashable, drifts silently (pstack needs maintain-verification-skill); our digest covers only files with content somebody runs" }
  - { id: S10, kind: decision, said: "the artifact is a program: it launches the real entry point, drives it, emits the JSON report the manifest cites; a harness is not the actor, and L3.GATE_HAS_FRESH_EVIDENCE refuses crediting it" }
  - { id: S11, kind: decision, said: "it comes from an interview, not a template: `interview/decisions.yaml` and `sf init --answers` already turn answers into configuration, and a second mechanism is a second thing to keep true" }
  - { id: S12, kind: decision, said: "observable criteria land in policy as `required_assertions`, which L3.GATE_COVERS_THE_PLAN reads, not in a prose directory it cannot read" }
  - { id: S13, kind: decision, said: "the harness sits inside the gate's activation paths, so editing how the product is driven expires the evidence and re-running is the only fix" }
  - { id: S14, kind: code, ref: ".software-factory/evidence/adoption-scenario.sh", said: "hand-written and correct: walks the README quickstart on an untuned repository, writes one violation, checks the build refuses it; it is the generator's test" }
  - { id: S15, kind: decision, said: "the generator on this repository must emit a harness reporting the same four `adoption` assertions, sealed green by `sf seal adoption`; failing that, it writes runbooks and is not a feature" }
  - { id: S16, kind: decision, said: "the swap is sf's own end-to-end test: adoption proven by a harness the tool produced, running the tool, on a codebase that never heard of it" }
  - { id: S17, kind: decision, ref: "docs/method.md", said: "decided: the existing factory-evidence skill, not a subcommand, which would need a language-neutral 'launch this product' the tool cannot acquire by parsing" }
  - { id: S18, kind: decision, ref: "skills/factory-evidence/SKILL.md", said: "the skill reads the repository, asks the facts code cannot reveal, writes harness and gate configuration when none exists, otherwise re-runs the existing harness when evidence goes stale" }
touches:
  - skills/factory-evidence/SKILL.md
  - .software-factory/evidence/adoption-scenario.sh
  - .software-factory/policy.yaml
  - docs/method.md
  - src/init.rs
  - interview/decisions.yaml
  - src/fixtures.rs
criteria:
  - { id: A1, statement: "the subcommand-or-skill decision is written into `docs/rules.md` or `docs/method.md` before anything ships", proof: "unspecified:the product-surface decision is recorded in docs/method.md and requires human review rather than an executable check", sources: [S4, S17, S18], done: true }
  - { id: A2, statement: "the generator, run against this repository, produces a harness emitting a report carrying all four `adoption` assertions", proof: "test:.software-factory/evidence/adoption-scenario.sh", sources: [S5, S6, S7, S8, S9, S10, S11, S12, S14, S15], done: true }
  - { id: A3, statement: "the generated harness replaces the hand-written `.software-factory/evidence/adoption-scenario.sh`, and its path sits inside the `adoption` gate's activation paths so editing it expires the evidence", proof: "test:.software-factory/evidence/adoption-scenario.sh", sources: [S13, S16], done: true }
  - { id: A4, statement: "`sf check --rule L3.GATE_HAS_FRESH_EVIDENCE` is green on the manifest sealed against the generated report, and red when `src/init.rs` changes without re-running the harness", proof: "test:.software-factory/evidence/adoption-scenario.sh; a scratch copy mutated src/init.rs and produced the expected stale-evidence finding", sources: [S1, S13], done: true }
  - { id: A5, statement: "the generator runs against an untuned repository whose product has a real startup dependency, and its harness reports failed when that product is broken", proof: "deferred:no corpus with a startup dependency is chosen yet", sources: [S6, S7, S16] }
  - { id: A6, statement: "an adopter enabling L3 reaches a first sealed gate without hand-writing the policy entry, the manifest, the report and the harness", proof: "deferred:`sf init` writes `gates: {}` and the interview has no answer that enables L3", sources: [S2, S3, S11] }
  - { id: A7, statement: "if a check ships with this, `sf verify` proves it fires on its own mutation fixture", proof: "deferred:whether any check ships is undecided above", sources: [S17] }
crosses:
  - { mechanism: skills, answer: works, proof: "test:.software-factory/evidence/adoption-scenario.sh (A2, A3)" }
  - { mechanism: init-and-interview, answer: works, proof: "deferred:A6" }
  - { mechanism: mutation-fixtures, answer: works, proof: "deferred:A7, only if a check ships" }
  - { mechanism: evidence, answer: works, proof: "test:.software-factory/evidence/adoption-scenario.sh (A4)" }
cut:
  - "a markdown runbook or feature directory as the artifact: refused (S9, S12)"
  - "a template that starts somebody's product, or an `sf` subcommand: refused (S11, S17)"
  - "requiring a gate to name a runnable harness: a floor under L3, owned by the shipped L3 proof-budget work; not split across two plans"
  - "separating a doctor step from driving: undecided, one worked example folds them and is not enough"
  - "a browser driver, sandbox or seed data: a tool with that opinion is a tool for one stack"
  - "touching or softening L3's five clauses: this only adds the missing producer in front of them"
  - "the generator writing the manifest's `actor`: it would credit runs to itself"
exit: "this repository's `adoption` evidence comes from a tool-generated harness, the gate is sealed against its report with `sf check` green, and the generator run on an untuned repository writes a harness whose report goes red when that product breaks"
---
# The gate assumes a way to run the product

L3 verifies a report of a run, but nothing in sf produces that run or checks a way to produce one exists. The one worked harness is a script kept by hand; an adopter meets it as a sentence in a skill.
