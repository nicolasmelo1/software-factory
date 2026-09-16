# External data is checked, not cast

Issue #56 describes a boundary that looks typed only because the author told
the compiler to stop asking: an external value is cast to a concrete shape and
fields are read through it without runtime validation. The catalog can match a
cast as a node today, but `shape` only reports the node and its placement
(`src/catalog.rs:47-58`, `:88-95`). It cannot follow the value, enumerate the
fields later read through it, or decide whether a guard dominates each read.
A text-pattern ban would report every cast, including the boundaries that do
validate, which is the false-positive failure the issue measured.

The nearest multi-query engine is `forwarder`: it captures two local shapes and
joins them by names inside one file (`src/checks/forwarder.rs:26-49`,
`:61-90`). That proves the extension point is a typed check kind with a
comparison queries alone cannot express. This plan adds the first local
flow-sensitive engine rather than pretending one tree-sitter match is a data
flow analysis.

## What changes

**A boundary-validation check kind.** Per-language specs capture four things:
an external source, the narrowing cast or assertion, field reads through the
narrowed value, and guards/validator calls. The engine joins captures inside one
function, builds the function's control-flow relation, and reports each field
read for which no validation dominates the use. It never claims whole-program
knowledge: aliases that escape the function and dynamic field names are
reported as unsupported analysis, not silently certified.

**Validation is explicit vocabulary.** Built-in guards cover language-native
type/null/presence checks. Policy may name validator functions whose successful
return establishes the whole value or named fields. A truthiness guard,
optional-chain fallback, schema parser and helper call are tested separately;
a helper is not accepted merely because its name sounds like validation.

**TypeScript is the first honest adapter.** The issue and measured corpus are
TypeScript-shaped, and `Lang` already carries TypeScript and TSX through one
rule surface (`src/lang.rs:35-45`). Other languages remain absent until their
boundary and narrowing vocabulary is measured; a language with no spec is not
reported as covered.

**Corpus measurement gates enablement.** The mutation includes one unsafe cast
and adjacent safe forms. Before the rule is enabled by default, its findings
are run over the repository that produced issue #56 and classified. The rule
ships enabled only when it finds the known defect, stays quiet on the two known
validated casts, and the measurement is recorded in the rule prose.

## Acceptance criteria

- [ ] A field read through a concrete cast of request, message, parsed document
      or `unknown` input is reported when no dominating validation establishes
      that field
      (proof: test:src/checks/boundary_validation.rs)
- [ ] Native type/presence guards, configured validator calls and schema parse
      results clear only the reads they actually establish
      (proof: test:src/checks/boundary_validation.rs)
- [ ] A guard on one branch does not certify a read reachable from another,
      and a guard after the read does not certify the earlier use
      (proof: test:src/checks/boundary_validation.rs)
- [ ] Aliases that escape local analysis and dynamic field reads are rendered
      as unsupported rather than counted as safe
      (proof: test:src/checks/boundary_validation.rs)
- [ ] The TypeScript fixture trips on the unsafe cast and stays quiet on the
      guarded and validator-backed forms beside it
      (proof: test:src/fixtures.rs)
- [ ] The measured source repository yields the known true finding and no
      finding on either already-validated cast before default enablement
      (proof: unspecified:the source repository and adjudication live outside
      this repository)

**Exit condition:** `sf check` identifies a field trusted through an external
TypeScript cast only when no dominating check or declared validator establishes
it, catches the measured defect, and does not turn every cast into advice a
maintainer learns to ignore.
