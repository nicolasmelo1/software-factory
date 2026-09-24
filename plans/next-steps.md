# Next steps

The execution order. One table, short on purpose: this is the file to reread
weekly, and the file an agent reads to know what is next.

A plan not listed here is written, valid, and off the critical path until its
precondition exists. Park it below rather than deleting it.

Rows 8 to 12 convert issues #55 to #57 and #54 after grounding them in the
current check engines. The runtime-pin preflight comes first because every
later result depends on the process being the one the repository declared.
Boundary validation follows because it introduces the local control-flow and
dominance vocabulary guard agreement reuses. Record-field consistency is an
independent field-set relation. Issue #54 becomes two rows rather than one:
helper adoption groups sibling functions by what they consume; guard agreement
compares callers by what dominates them, and neither exit condition proves the
other.

Rows 15 to 17 follow the split of #63, which adds a rule that asks a model
whether a gate's report supports a claim. They start once that rule and its
backend protocol have landed. A verdict becomes sealed evidence first, because
the other two are checks over it; the judge is proven to fail next, because a
backend nobody proved can say no makes every verdict worthless; and the backend
suite comes last, because it needs both to have something to prove end to end.

| # | Work | Exit condition |
| --- | --- | --- |
| 1 | [The gate assumes a way to run the product](the-gate-assumes-a-way-to-run-the-product.md) | This repository's `adoption` evidence comes from a harness the tool generated, the gate is sealed against it and `sf check` is green, and the same generator run against a repository nobody tuned it for writes a harness whose report goes red when that product is broken. |

| 2 | [Expand the language adapters](expand-language-adapters.md) | A repo in a new language runs `sf init`, `sf verify` is green, and `sf check` finds something a maintainer of that language agrees is real. |

| 3 | [The four L0 structural rules assume an import statement](structural-rules-assume-an-import-statement.md) | `sf check` on a Rails repository with `languages: [ruby]` reports a real cross-layer finding read from a constant reference rather than a `require` statement, with zero findings on Rails' own base-class inheritance. |

| 4 | [Scaffolds are proven in micro-sandboxes](scaffolds-are-proven-in-micro-sandboxes.md) | A repository declares a scaffold, `sf scaffold` writes a router, service and query that `sf check` passes unedited, and `sf verify` goes red both when that output is mutated to violate a governing rule and when every rule over those paths is removed. |

| 5 | [Rule packs for third-party APIs and libraries](third-party-rule-packs.md) | `sf pack add <name>@<version>` vendors a versioned set of rules with their fixtures into `.software-factory/rules/`, refuses any pack whose fixtures do not trip its own rules, and the installed rules deactivate with a finding when the dependency's major version moves. |

| 6 | [Every fixture carries its repair](every-fixture-carries-its-repair.md) | `sf verify` reports for every enabled rule both that its mutation trips the rule and that its repair clears it, and a finding on a rule nobody has been stuck on renders a worked example that came out of the binary. |

| 7 | [An overlay run says what it cannot know](an-overlay-run-says-what-it-cannot-know.md) | An overlay run over a repository carrying no `.software-factory/` reports the findings that code earns and nothing else, every rule whose state travels with the policy says so per rule, and each refusal the flag owes carries a test. |

| 8 | [The running toolchain matches the pin](the-running-toolchain-matches-the-pin.md) | A repository that pins a runtime cannot run any other factory check under a different one without `sf check` stopping first and naming the pin and the process that disagree; a repository that pins nothing is not told what to use. |

| 9 | [External data is checked, not cast](external-data-is-checked-not-cast.md) | `sf check` identifies a field trusted through an external TypeScript cast only when no dominating check or declared validator establishes it, catches the measured defect, and does not turn every cast into advice a maintainer learns to ignore. |

| 10 | [A reconciled record compares what it wrote](a-reconciled-record-compares-what-it-wrote.md) | A configured TypeScript reconciliation path cannot call a record unchanged while ignoring a statically known non-identity field its own creation wrote, and uncertainty in the field set is visible rather than reported as coverage. |

| 11 | [Siblings share the helper](siblings-share-the-helper.md) | A measured sibling group cannot leave one handler open-coding a decision the others take from a shared helper without `sf check` naming the helper, the group and the outlier, and the rule has proved it fires on real code rather than only on its own fixture. |

| 12 | [Calls to one capability share the guard](calls-to-one-capability-share-the-guard.md) | Callers of one resolvable capability cannot silently disagree about the guard that enables it, a point fix does not hide the next unguarded path, and unsupported control flow is counted rather than called safe. |

| 13 | [A property is a proof with a falsifier](a-property-is-a-proof-with-a-falsifier.md) | A plan cites a statement rather than an example, the citation names the mutation that statement rejects, and removing that mutation turns the plan's own gate red. |

| 14 | [A hazard tool is proven to fail](a-hazard-tool-is-proven-to-fail.md) | A repository whose performance guard cannot fail is told so by `sf verify`, with the command it ran and the exit code it got, and every other hazard tool it declares is proved to go red on a defect planted under it. |

| 15 | [A verdict is evidence](a-verdict-is-evidence.md) | A repository with marked claims runs `sf judge` once, commits the sealed verdicts, and from then on `sf check` is deterministic and offline for this rule, and goes red the moment a claim sentence or the report it cites changes without being judged again. |

| 16 | [A judge is proven to fail](a-judge-is-proven-to-fail.md) | A repository whose judge approves everything is told so by `sf verify` before any claim is judged, and relaxing what the judge accepts is a policy change the L2 rules refuse rather than a string edit `sf lock` absorbs. |

| 17 | [Any typed decision model can judge](any-typed-decision-model-can-judge.md) | This repository's own tests prove the rule end to end with no account, no key and no tokens spent, and switching a consumer from Jev to a local model is a change to one backend command with no change to the question file or the sealed verdict format. |

## Parked

Nothing is parked. Rule packs waited on version-conditional activation, which
shipped in `927e8e2`, so they join the order above.