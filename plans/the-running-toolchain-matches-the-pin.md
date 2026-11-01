---
sources:
  - { id: S1, kind: ticket, ref: "#55", said: "a repository pins its runtime, the shell runs another, and the first visible disagreement is a regenerated lockfile or artifact" }
  - { id: S2, kind: code, ref: "src/checks/toolchain.rs", said: "`run` is an L6 wiring assertion searching committed runners for a hazard tool, its policy shape a language-to-command map (`Options.tools` in src/policy.rs); reuse would make one name mean scanner-invoked and pin-honoured" }
  - { id: S3, kind: code, ref: "src/checks/mod.rs", said: "`run_all` walks enabled instances in map order and dispatches each independently; no preflight says every later result was measured by the wrong runtime" }
  - { id: S4, kind: decision, said: "a runtime mismatch stops the run before a lock, generated artifact, fixture or command check can produce secondary findings" }
  - { id: S5, kind: decision, said: "a distinct runtime-pin kind pairs a declaration with the command reporting the running version: `.nvmrc`/`.node-version`, `.python-version`, `rust-toolchain.toml`, `.tool-versions`, `go.mod`, Node `engines`" }
  - { id: S6, kind: decision, said: "exact pins compare exactly; ranges use the declaration's own range semantics; an uninterpretable file is a finding, not a guessed match" }
  - { id: S7, kind: decision, said: "sf check runs enabled runtime-pin instances before `run_all` dispatches ordinary checks; one mismatch reports pin path, declared value, executable and observed value, then stops" }
  - { id: S8, kind: decision, said: "a repository with no supported pin is silent: it stated no version for sf to enforce. An overlay runs the same preflight, the subject being target code" }
  - { id: S9, kind: decision, said: "version commands are fixed argv per adapter, never a shell or policy command; `.tool-versions` runtimes are checked independently; a missing executable is named missing, not a version disagreement" }
  - { id: S10, kind: decision, said: "the rule has prose, a mutation whose reported runtime differs from its pin and a repair that agrees; tests inject command output, independent of the host's runtimes" }
touches:
  - src/checks/runtime_pin.rs
  - src/checks/mod.rs
  - src/catalog.rs
  - catalog/L2/running-toolchain-matches-the-pin.yaml
  - src/fixtures.rs
  - src/checks/toolchain.rs
criteria:
  - { id: A1, statement: "a supported exact pin differing from the running executable stops `sf check` before any ordinary rule runs, naming the pin, executable, declared and observed version", proof: "test:src/checks/runtime_pin.rs", sources: [S1, S3, S4, S5, S7], done: true }
  - { id: A2, statement: "a matching exact pin and a satisfied supported range pass; a malformed declaration and a missing executable produce distinct, actionable findings", proof: "test:src/checks/runtime_pin.rs", sources: [S6, S9], done: true }
  - { id: A3, statement: "a repository carrying none of the supported pin declarations receives no runtime-pin finding", proof: "test:src/checks/runtime_pin.rs", sources: [S8], done: true }
  - { id: A4, statement: "every supported declaration format has a mutation and a repair whose command output is injected rather than read from the test host", proof: "test:src/fixtures.rs", sources: [S10], done: true }
  - { id: A5, statement: "the existing L6 `toolchain` rules keep their present meaning and output unchanged", proof: "test:src/checks/toolchain.rs", sources: [S2], done: true }
crosses:
  - { mechanism: toolchain, answer: works, proof: "test:src/checks/runtime_pin.rs (A1-A3); test:src/checks/toolchain.rs (A5)" }
  - { mechanism: inertness, answer: works, proof: "test:src/checks/runtime_pin.rs (A3): no pin is silent" }
  - { mechanism: overlay, answer: works, proof: "deferred:no criterion covers it; S8 says an overlay runs the same preflight" }
  - { mechanism: catalog, answer: works, proof: "test:src/fixtures.rs (A4)" }
  - { mechanism: mutation-fixtures, answer: works, proof: "test:src/fixtures.rs (A4)" }
guarantee:
  - { holds: "no factory check runs under a runtime that disagrees with a supported pin", not_against: "a repository that pins nothing: it is not told what to use" }
failures:
  - { step: "runtime-pin preflight", when: "the running version disagrees with the pin", goes_to: "names pin, executable, declared and observed version, then stops before ordinary checks" }
  - { step: "runtime-pin preflight", when: "a declaration is malformed or the executable missing", goes_to: "a distinct finding, never a guessed match or a version disagreement" }
cut:
  - "reusing the L6 `toolchain` kind: refused, one name would mean two things (S2)"
  - "shell or policy-provided version commands: refused (S9)"
exit: "a repository pinning a runtime cannot run any other factory check under a different one without `sf check` stopping first and naming the pin and the process that disagree; a repository pinning nothing is not told what to use"
---
# The running toolchain matches the pin

Issue #55: a repository pins one runtime and the shell runs another, and the first sign is a regenerated lockfile. Every later finding was measured by the wrong runtime.
