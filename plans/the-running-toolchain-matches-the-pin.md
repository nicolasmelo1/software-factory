# The running toolchain matches the pin

Issue #55 records a cheap failure with an expensive symptom: a repository pins
its runtime, the shell runs another one, and the first visible disagreement is
a regenerated lockfile or artifact. The current `toolchain` check is not this
check. It is an L6 wiring assertion that searches committed runners for a
hazard tool (`src/checks/toolchain.rs:1-12`, `:21-55`), and its policy shape is a
language-to-command map (`src/policy.rs:59-62`). Reusing that kind would make
one name mean both “CI invokes a scanner” and “this process honours a version
pin.”

The check belongs before source rules. `run_all` currently walks enabled policy
instances in their map order and dispatches each independently
(`src/checks/mod.rs:79-108`); there is no preflight whose failure says every
later result was measured by the wrong runtime. A runtime mismatch must stop
the run before a lock, generated artifact, fixture or command check can produce
secondary findings.

## What changes

**A distinct runtime-pin check kind.** Add a source-facing check kind whose
adapters pair a declared version with the command that reports the running
version. The first supported declarations are `.nvmrc`/`.node-version`,
`.python-version`, `rust-toolchain.toml`, `.tool-versions`, `go.mod`, and the
Node `engines` field. Exact pins compare exactly; ranges use the declaration's
own range semantics. A file whose syntax cannot be interpreted is a finding,
not a guessed match.

**One preflight, derived from enabled rules.** `sf check` discovers enabled
runtime-pin instances and runs them before `run_all` dispatches ordinary
checks. One mismatch reports the pin path, declared value, executable and
observed value, then stops. A repository with no supported pin is silent: it
did not state a version for `sf` to enforce. An overlay runs the same preflight
because the subject is target code, not repository-local factory state.

**No shell-dependent parsing.** Version commands are fixed by the adapter and
run as argv, not through a shell or policy-provided command. Multi-runtime
`.tool-versions` entries are checked independently so one matching runtime
cannot hide another mismatch. Missing executables are named as missing rather
than rendered as version disagreements.

**The catalog proves both directions.** The rule has prose, a mutation fixture
whose reported runtime differs from its pin, and a repair whose pin and runtime
agree. Tests inject command output; they do not depend on whichever runtimes
happen to be installed on the machine running the suite.

## Acceptance criteria

- [x] A supported exact pin that differs from the running executable stops
      `sf check` before any ordinary rule runs and names the pin, executable,
      declared version and observed version
      (proof: test:src/checks/runtime_pin.rs)
- [x] A matching exact pin and a satisfied supported range pass, while a
      malformed declaration and a missing executable produce distinct,
      actionable findings
      (proof: test:src/checks/runtime_pin.rs)
- [x] A repository carrying none of the supported pin declarations receives
      no runtime-pin finding
      (proof: test:src/checks/runtime_pin.rs)
- [x] Every supported declaration format has a mutation and a repair whose
      command output is injected rather than read from the test host
      (proof: test:src/fixtures.rs)
- [x] The existing L6 `toolchain` rules keep their present meaning and output
      unchanged
      (proof: test:src/checks/toolchain.rs)

**Exit condition:** a repository that pins a runtime cannot run any other
factory check under a different one without `sf check` stopping first and
naming the pin and the process that disagree; a repository that pins nothing
is not told what to use.
