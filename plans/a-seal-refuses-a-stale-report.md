---
base: 6914586d0c97e6185c798717b364f6d7c3c914ae
sources:
  - { id: S1, kind: friction, said: "a consuming repository re-sealed four gates over reports produced before its last build; `sf seal` succeeded, `L3.GATE_HAS_FRESH_EVIDENCE` went green, and none of that evidence had seen the code it certified" }
  - { id: S2, kind: code, ref: "src/checks/evidence.rs", said: "`seal` recomputes `implementation_sha256` from the activation paths and each `report_sha256` from the file on disk; nothing asks which implementation a report was produced against" }
  - { id: S3, kind: code, ref: "src/checks/evidence.rs", said: "`check_manifest_identity` compares the manifest's implementation digest with the current one, and that digest is the one `seal` just wrote, so any re-seal clears `:stale` whether or not anything was re-run" }
  - { id: S4, kind: reference, ref: "skills/factory-evidence/SKILL.md", said: "\"Stale means re-run, not re-seal ... a lie with a hash on it\": the rule exists only as prose an agent can skip" }
  - { id: S5, kind: code, ref: "src/checks/evidence.rs", said: "`required_assertions` is the union of policy and manifest, so a manifest can add an assertion but never drop one; when a scenario is replaced, the run entry itself has to be replaced before sealing, and nothing says so" }
  - { id: S6, kind: decision, said: "a report names the implementation it ran against in a top-level `implementation_sha256`; a harness learns it from `sf seal <gate> --digest`, which prints the current digest and writes nothing" }
  - { id: S7, kind: decision, said: "`sf seal` refuses, and writes nothing, when any run's report is missing that field or names a digest other than the current one; the refusal names the run, the report and both digests" }
  - { id: S8, kind: decision, said: "`L3.GATE_HAS_FRESH_EVIDENCE` reports `<gate>:<scenario>:report-stale` when a report's digest differs from the manifest's, so a manifest edited by hand cannot get around the seal" }
  - { id: S9, kind: decision, said: "the field is required from the release that ships this, not phased in: the version pin makes the upgrade deliberate, and an optional field would leave the hole open for everyone who never adds it" }
  - { id: S10, kind: code, ref: "src/init.rs", said: "`sf init` writes the harness a gate starts from, so the harness it writes has to stamp the digest or every new gate fails its first seal" }
touches:
  - src/checks/evidence.rs
  - src/main.rs
  - src/init.rs
  - catalog/L3/gate-has-fresh-evidence.yaml
  - docs/rules.md
  - skills/factory-evidence/SKILL.md
  - .software-factory/evidence/adoption-scenario.sh
criteria:
  - { id: A1, statement: "`sf seal` refuses a gate when a run's report names an implementation digest other than the current one, names the run, the report and both digests, and leaves the manifest byte-for-byte unchanged", proof: "test:src/checks/evidence.rs", sources: [S1, S2, S7] }
  - { id: A2, statement: "`sf seal` refuses a report carrying no `implementation_sha256`, with the same no-write guarantee", proof: "test:src/checks/evidence.rs", sources: [S7, S9] }
  - { id: A3, statement: "`sf seal <gate> --digest` prints the digest `seal` would record, and nothing on disk changes", proof: "test:src/checks/evidence.rs", sources: [S6] }
  - { id: A4, statement: "a manifest whose implementation digest is current but whose report names an older one produces `report-stale` from `sf check`", proof: "test:src/checks/evidence.rs", sources: [S3, S8] }
  - { id: A5, statement: "the harness `sf init` writes stamps the digest into its report, and that report seals on the first try", proof: "test:src/init.rs", sources: [S10] }
  - { id: A6, statement: "this repository's `adoption` harness stamps the digest, its gate is re-run and re-sealed, and `sf check` is green", proof: "test:.software-factory/evidence/adoption-scenario.sh", sources: [S9] }
  - { id: A7, statement: "the factory-evidence skill and the rule's `fix` describe the stamping, the refusal, and how to replace a run entry when a scenario is superseded, because the assertion union never drops one", proof: "unspecified:prose about a procedure; read in review", sources: [S4, S5] }
crosses:
  - { mechanism: evidence, answer: works, proof: "test:src/checks/evidence.rs (A1-A4)" }
  - { mechanism: commands, answer: works, proof: "test:src/checks/evidence.rs (A3): `--digest` is a flag on the existing `seal`, so prose that names it names a real command" }
  - { mechanism: init-and-interview, answer: works, proof: "test:src/init.rs (A5)" }
  - { mechanism: catalog, answer: works, proof: "test:src/checks/evidence.rs (A4): the rule id is unchanged, only its finding set grows" }
  - { mechanism: generated-docs, answer: works, proof: "`sf docs --check` after the rule's prose changes" }
  - { mechanism: skills, answer: works, proof: "unspecified:A7 is prose read in review; the skill is already registered in src/skills.rs" }
guarantee:
  - { holds: "`sf seal` never records a report produced against an implementation other than the one on disk, and `sf check` never stays green over one", not_against: "a harness that stamps the digest without having run against that code, or runs against build output not rebuilt from the sources the digest covers: the digest identifies the activation paths, not the artifact" }
failures:
  - { step: "sf seal", when: "a report names a different digest or none", goes_to: "refuses, names the run, report and both digests, writes nothing" }
  - { step: "sf check", when: "a sealed report names a digest other than the manifest's", goes_to: "`report-stale` finding on the gate" }
cut:
  - "inferring staleness from file modification times: refused, a checkout resets them and CI never sees the order things happened in"
  - "an optional field or a warning-only phase: refused (S9)"
  - "proving the harness really ran against the build it names"
exit: "a gate cannot be sealed, and `sf check` cannot go green, over a report that was produced against an implementation other than the one on disk; re-sealing without re-running fails and says to re-run"
---
# A seal refuses a stale report

`sf seal` recomputes digests and nothing else. A consuming repository re-sealed four gates over reports produced before its last build, `L3.GATE_HAS_FRESH_EVIDENCE` went green, and none of that evidence had seen the code it certified. The skill already forbids this ("stale means re-run, not re-seal"), but the binary does not enforce it.

The fix is to bind each report to the implementation it ran against. The harness stamps the digest `sf seal <gate> --digest` prints, `seal` refuses any report whose stamp is absent or different, and the L3 check refuses the same mismatch, so editing the manifest by hand does not get around it either.

The second half is documentation. Because `required_assertions` is a union that can add but never drop, superseding a scenario means replacing its run entry before sealing. The skill now says so, next to the refusal it explains.
