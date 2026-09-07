# The gate: L3 in practice

`L3.GATE_HAS_FRESH_EVIDENCE` is the method's sharpest edge and the reason the
rest of it exists.

```yaml
# .software-factory/policy.yaml
gates:
  checkout:
    activation: ["src/checkout/**"]
    evidence: "evidence/checkout.json"
```

1. **Activation comes from touched paths.** Not a label, not a checkbox, not a
   sentence in a pull request. Touch `src/checkout/**` and the gate is on.
2. **The manifest is re-verified, never trusted.** `sf` re-reads the referenced
   report, recomputes its SHA-256, and re-checks every required assertion in
   the raw report. A summary cannot assert a pass the report never contained.
3. **An `unsupported` assertion is not a pass.** The most common way a green
   gate proves nothing is an assertion the harness could not evaluate being
   counted as one that succeeded.
4. **Evidence expires when the code moves.** The manifest records a digest of
   the activation paths. Change the implementation and the evidence dies with
   it, instead of quietly certifying something it never saw.
5. **The goal is checked for leaked answers.** A goal naming the source tree is
   a replay recipe, not a customer asking for something.

```sh
sf seal checkout   # recompute every digest from what is actually on disk
```

`seal` only recomputes digests — it cannot launder a failing report into a
passing one.
