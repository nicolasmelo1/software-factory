# Reviewing a pull request in software-factory

A change that implements a plan says so in its title or body, and the plan is a
file in `plans/` (or `docs/design/` once delivered) that opens with a YAML
declaration. Read that declaration before the diff. It is the scope of the
review.

- **`cut` and `guarantee[].not_against` are out of scope.** A finding they
  cover is not a defect of this change. Do not report it.
- **A `crosses` entry answered `works` with no test behind its `proof` is a
  finding.** So is one answered `refused` where the diff does not refuse it.
- **A mechanism in [`docs/mechanisms.yaml`](../docs/mechanisms.yaml) whose
  `paths` the diff changes, and that the plan's `crosses` does not answer, is a
  finding.** Name the mechanism and the concrete scenario that breaks.
- **A file the diff changes outside the plan's `touches`**, other than tests
  and generated files (evidence digests, mutation fixtures, locks, the ratchet,
  generated README tables and docs/rules.md sections), is worth one question: why is it here.
- **A `failures` entry the code does not honour is a finding.**

Report a defect with the file and line, the concrete state that breaks it and
what it does wrong. Do not report style, naming or comment wording.
