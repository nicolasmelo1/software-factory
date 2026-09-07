# The seven layers

The numbers are identity, not sequence: they group rules by what they are
about. Which order to adopt them in is a different question, answered in
[adopting a repository with history](../start/adopting-a-repo-with-history.md).

<!-- sf:generated layer-index -->
| | Layer | What it checks | Shipped | Enabled here |
| :-- | :-- | :-- | --: | --: |
| **L0** | Shape | where things live | 4 | 2 |
| **L1** | Grain | how the code reads | 6 | 6 |
| **L2** | Contract | no drift from the source of truth | 7 | 6 |
| **L3** | Effect | a real actor achieved the outcome | 2 | 2 |
| **L4** | Cadence | docs, plans and rules stay attached | 8 | 8 |
| **L5** | Meta | the guardrail is proven to fire | 2 | 2 |
| **L6** | Hazard | the defect classes this repository hunts | 9 | 8 |

<!-- sf:end layer-index -->

Run `sf catalog` for the rules, `sf explain <RULE>` for the reasoning behind any
one of them.
