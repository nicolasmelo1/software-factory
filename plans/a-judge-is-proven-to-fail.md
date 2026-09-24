# A judge is proven to fail

Every rule in the catalog carries a mutation, and `sf verify` fails if the
mutation does not trip it. The mutation #63 ships for
the claim-support rule sets `run: "exit 1"`. That proves `sf`
reports a failing command. It says nothing about the judge, and a judge that
answers `supports` to everything passes every run forever.

The proposal knows this and asks for a twin request with the criteria swapped,
run by hand. By hand means never, which is the argument this repository makes
for mutation fixtures in the first place.

It has the same shape as
[a hazard tool is proven to fail](a-hazard-tool-is-proven-to-fail.md): the
check delegates to something, and nothing asks whether that something can say
no.

## What changes

- `sf verify` builds the twin itself. For each question the policy expects, it
  sends the same state with the criteria of the expected label and a failing
  label swapped, and requires the backend to land on the expected label for the
  real request and away from it for the twin. A backend that passes both is
  refused as unable to fail. A backend that fails both is refused as unable to
  pass.
- The twin runs against a planted claim with a planted report, so it needs no
  real claim in the repository and works on the first day.
- `expect` and `min_confidence` become structured options of the rule rather
  than flags inside a `run` string. Today `L2.POLICY_ONLY_TIGHTENS` counts
  `exclude`, `scope`, `max` and the `forbidden_*` lists
  (`src/checks/tightening.rs`) and cannot see into a command, so lowering the
  floor from `0.8` to `0.1` only trips `L2.FACTORY_CONFIG_IS_LOCKED`, which
  `sf lock` clears. As structured options, a lower floor or a wider set of
  accepted labels is a loosening that rule refuses.
- The finding separates the two failures #63 already names: the wrong label is
  a bad claim, the right label below the floor is usually a badly posed
  question or thin evidence. They get different messages because they get
  different fixes.

## What this does not claim

The twin proves the backend responds to the criteria. It does not prove the
criteria are well written, and a question whose options overlap can still pass
both sides by accident. That remains review work on a committed file.

## Acceptance criteria

- [ ] `sf verify` refuses a backend that answers the expected label for both
      the request and its twin
      (proof: test:src/judge.rs)
- [ ] `sf verify` refuses a backend that answers against the expected label
      for both
      (proof: test:src/judge.rs)
- [ ] The rule's mutation fixture is an always-agreeing backend, and
      `sf verify` goes red on it
      (proof: test:src/fixtures.rs)
- [ ] Lowering `min_confidence` or adding a label to `expect` is reported by
      `L2.POLICY_ONLY_TIGHTENS`, and raising or narrowing them is not
      (proof: test:src/checks/tightening.rs)
- [ ] A wrong label and a right label below the floor produce distinct finding
      messages
      (proof: test:src/checks/judgment.rs)

**Exit condition:** a repository whose judge approves everything is told so by
`sf verify` before any claim is judged, and relaxing what the judge accepts is
a policy change the L2 rules refuse rather than a string edit `sf lock` absorbs.
