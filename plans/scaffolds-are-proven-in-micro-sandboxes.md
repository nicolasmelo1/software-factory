---
sources:
  - { id: S1, kind: reference, ref: "Rails generators (2005)", said: "settled generation; a generator normally cannot stay honest, and this tool can" }
  - { id: S2, kind: code, ref: "src/main.rs", said: "sf init scaffolds 'policy, docs, CI, hooks and mutation fixtures'; sf fixtures writes one fixture per enabled rule; sf verify proves each rule fires on its own" }
  - { id: S3, kind: friction, ref: "automate-my-work", said: "26 rules, all proven; a local rule's fixture is three files (policy.yaml, the rule, one violating source): already a self-contained micro-sandbox" }
  - { id: S4, kind: code, ref: "templates/", said: "four templates, all rule templates: generate-then-prove is aimed only at rules about rules" }
  - { id: S5, kind: decision, said: "a scaffold lives in `.software-factory/scaffolds/<name>/` beside `.software-factory/rules/`, in the repository whose conventions it encodes; sf ships the mechanism, never shapes" }
  - { id: S6, kind: decision, said: "a new rule makes sf verify prove, per scaffold, that its output passes the rules governing those paths and a mutated copy fails them, the L5.NO_INERT_RULE argument" }
  - { id: S7, kind: decision, said: "rule and scaffold are two statements of one convention; the build fails the moment they disagree, whichever is wrong, which stops generator rot" }
  - { id: S8, kind: decision, said: "agents are not told via a prompt list (tokens per call, rots): a rule fires on hand-written mismatching code and its `fix` names the scaffold command; discovery is a gate finding" }
  - { id: S9, kind: decision, said: "a scaffold earns upkeep only where the shape repeats; its shape is a floor, never a ceiling; declining stays possible with a reason recorded; a rule requires the shape only where genuinely required" }
  - { id: S10, kind: decision, said: "a scaffold nobody chooses is deleted, and the usage record says which" }
touches:
  - src/scaffold.rs
  - src/main.rs
  - src/verify.rs
  - src/checks/cadence.rs
  - src/catalog.rs
  - catalog/**
criteria:
  - { id: A1, statement: "a scaffold definition in `.software-factory/scaffolds/` emits its files into a target repository, and nothing outside the paths it declares", proof: "assertion:scaffold.writes_only_what_it_declares", sources: [S2, S4, S5] }
  - { id: A2, statement: "`sf verify` refuses a scaffold whose output violates a rule governing the paths it writes, so scaffold and rule cannot drift apart silently", proof: "test:src/scaffold.rs", sources: [S1, S6, S7] }
  - { id: A3, statement: "`sf verify` refuses a scaffold whose output no rule can reject, on the argument `L5.NO_INERT_RULE` makes about a rule nothing can trip", proof: "test:src/scaffold.rs", sources: [S6] }
  - { id: A4, statement: "a micro-sandbox generated for a scaffold author runs the real tool against generated output without a checkout of the host repository", proof: "assertion:sandbox.proves_generated_output_standalone", sources: [S3] }
  - { id: A5, statement: "a rule governing a scaffolded path names the scaffold command in its `fix`, so an agent writing the shape by hand is told what would have written it", proof: "test:src/checks/cadence.rs", sources: [S8] }
  - { id: A6, statement: "whether declining a scaffold is recorded in the repository or left to the caller is decided before any check ships", proof: "unspecified:a decision about where usage lives, which no check can validate, because the answer changes what the record is for", sources: [S9, S10] }
crosses:
  - { mechanism: commands, answer: works, proof: "test:src/checks/cadence.rs (A5): the fix names `sf scaffold`, a real command" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/scaffold.rs (A2, A3)" }
  - { mechanism: inertness, answer: works, proof: "test:src/scaffold.rs (A3)" }
  - { mechanism: plans, answer: n/a, why: "cadence.rs gains a check on rule `fix` prose; plan declaration and ordering are unchanged" }
  - { mechanism: catalog, answer: works, proof: "test:src/scaffold.rs (A2)" }
guarantee:
  - { holds: "a scaffold's output passes the rules governing its paths, and a mutated copy fails them", not_against: "a shape that genuinely does not repeat; declining stays possible" }
cut:
  - "shipping shapes, or a catalogue of somebody else's, in sf: refused (S5)"
  - "listing scaffolds in an agent's prompt: refused (S8)"
  - "a mandatory scaffold or a shape as ceiling: refused (S9)"
exit: "`sf scaffold <name>` writes a router, service and query `sf check` passes unedited; `sf verify` goes red when that output violates a governing rule, and when no rule governs its paths; hand-writing one draws a finding naming the command"
---
# Scaffolds are proven in micro-sandboxes

Every convention a rule checks is one something could have written, yet agents rewrite known shapes from scratch and the rule merely confirms the guess. A generator usually rots; here the rule and the scaffold prove each other.
