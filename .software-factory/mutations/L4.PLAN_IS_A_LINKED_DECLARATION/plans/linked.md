---
sources:
  - { id: S1, kind: decision, said: refunds post to the ledger }
touches: ["src/ledger/post.py"]
criteria:
  - { id: A1, statement: a refund posts to the ledger, proof: "test:tests/refund.py", sources: [S1] }
crosses:
  - { mechanism: ledger, answer: works, proof: "test:tests/refund.py" }
cut:
  - partial refunds
exit: refunds post
---
# Linked

Refunds never reached the ledger.
