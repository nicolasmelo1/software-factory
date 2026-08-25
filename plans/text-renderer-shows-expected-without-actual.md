# Text renderer shows `expected` even when `actual` is absent

`Report::text()` in `src/report.rs` only prints a finding's `expected`/`actual`
pair when both are set:

```rust
if let (Some(expected), Some(actual)) = (&finding.expected, &finding.actual) {
    out.push_str(&format!("       expected {expected}\n       actual   {actual}\n"));
}
```

`root_files()` in `src/checks/cadence.rs` (the check behind
`L4.ROOT_FILES_ARE_DECLARED`) sets only `.expected(...)` on its finding. The
JSON renderer has no such gate and always serializes whichever fields are
`Some`, so the data survives there; the text path — the one a human or an
agent reads first — silently drops the one string that says what to do about
the finding. Confirmed on this repository itself: dropping a stray file at
the root and running `sf check` prints only

```
    ZZTEMP-probe.md — `ZZTEMP-probe.md` is at the repository root but not declared
```

while `sf check --format json` for the same finding carries
`"expected": "an entry in .allowed-root-files, or somewhere with a lifecycle"`.
Acting on the text finding required disassembling the binary to find the
allowlist's filename.

This is not one check's bug. Of the 35 `.expected(` call sites across
`src/checks/*.rs`, 10 never pair it with `.actual(`: eight in
`src/checks/cadence.rs`, one in `src/checks/evidence.rs`, one in
`src/checks/lock.rs`. Every one of those ten findings loses its `expected`
line in text output today. Fixing `root_files()` alone would leave nine more
checks with the identical silent drop, so the fix belongs in the renderer:
print `expected` whenever it is `Some`, print `actual` whenever it is `Some`,
independently, keeping the existing combined layout for findings that set
both (nothing about that case changes — same two lines, same order, same
indentation).

Explicitly out of scope for this change: `sf verify`'s `Outcome` has no
`Serialize` derive and no format flag, and `sf explain`/`sf catalog` have no
machine-readable output path at all. Both are real gaps; neither shares a root
cause with this one, and bundling them would make one PR review three
unrelated surfaces.

**Exit condition:** a finding produced by any check that sets `expected`
without `actual` (or `actual` without `expected`) shows that field in
`sf check`'s text output, proven by the `L4.ROOT_FILES_ARE_DECLARED` mutation
fixture continuing to pass under `sf verify` and by `sf check` naming
`.allowed-root-files` for a stray root file in this repository.

## Acceptance criteria

- [x] `sf check`'s text output prints `expected` for a finding that sets
      `expected` alone, matching what `sf check --format json` already showed
      (proof: test:.software-factory/mutations/L4.ROOT_FILES_ARE_DECLARED/)
- [x] A finding that sets both `expected` and `actual` keeps rendering the
      existing two-line combined layout unchanged
      (proof: test:.software-factory/mutations/L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE/)
- [ ] `sf verify`'s `Outcome` gains a `Serialize` derive and a format flag
      (proof: deferred:tracked as a separate PR, not this one)
- [ ] `sf explain`/`sf catalog` gain a machine-readable output path
      (proof: deferred:tracked as a separate PR, not this one)
