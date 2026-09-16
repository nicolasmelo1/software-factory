# A reconciled record compares what it wrote

Issue #57 records a reconciliation defect that exists only in the difference
between two field sets: creation writes the record's declared shape, while the
later unchanged decision reads fewer fields and lets the rest drift forever.
The update is not the reference — it often writes exactly the same incomplete
subset the decision compared.

No current source check can state that relation. `shape` matches one target;
`forwarder` joins imported and forwarded names; `nested` compares containment
(`src/catalog.rs:88-121`). The dispatch reflects those local engines and has no
write/read field-set check (`src/checks/mod.rs:247-267`). This plan adds the
smallest relational engine that compares an insert with a reconciliation
decision in the same scope, rather than widening a pattern until it resembles
one.

## What changes

**A record-consistency check kind.** Per-language specs identify record writes,
the identity used to match an external record, and the branch that decides the
record is unchanged. The engine extracts static field names from the creation
write and fields read by the decision, subtracts the configured identity fields,
and reports the remainder. It points to both the creation and decision because
either alone looks correct.

**Creation is authoritative.** Updates are evidence about how the code mutates
a row, never the source of the expected set. Multiple creation sites are
compared separately; if they disagree on shape, that disagreement is reported
before a reconciliation result is claimed. Spreads, computed keys and helper
builders whose fields cannot be recovered make the instance unsupported, not
clean.

**Matching stays configurable and narrow.** A policy instance names the create
callee, lookup/reconcile scope and identity fields for one record family. The
first TypeScript adapter handles object-literal inserts and direct/member reads
inside one function or module-local reconciliation path. Cross-file provenance
waits for a symbol graph; this work does not imply one exists.

**A real corpus decides usefulness.** The mutation reproduces the payment/type
failure and carries its repair. The measured repository must produce the one
known finding and no false positives before the rule instance is enabled.

## Acceptance criteria

- [ ] An object-literal creation writing identity plus three data fields and an
      unchanged decision reading only two data fields reports the omitted field
      at both source locations
      (proof: test:src/checks/record_consistency.rs)
- [ ] Identity fields are subtracted explicitly, while update fields are never
      used as the expected record shape
      (proof: test:src/checks/record_consistency.rs)
- [ ] Adding the omitted field to the comparison clears the mutation fixture
      without changing the insert
      (proof: test:src/fixtures.rs)
- [ ] Divergent creation shapes, computed keys, spreads and unresolved builders
      are named as unsupported or ambiguous rather than rendered as a clean
      comparison
      (proof: test:src/checks/record_consistency.rs)
- [ ] The measured source repository yields the known payment-type finding,
      zero false positives, and clears when that field joins the decision
      (proof: unspecified:the source repository and adjudication live outside
      this repository)

**Exit condition:** a configured TypeScript reconciliation path cannot call a
record unchanged while ignoring a statically known non-identity field its own
creation wrote, and uncertainty in the field set is visible rather than
reported as coverage.
