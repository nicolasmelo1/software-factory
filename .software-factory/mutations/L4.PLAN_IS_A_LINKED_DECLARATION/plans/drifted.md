---
sources:
  - { id: S1, kind: decision }
touches: ["src/ledger/post.py"]
criteria:
  - { id: A1, statement: a refund posts to the ledger, proof: "test:tests/refund.py", sources: [S1] }
crosses:
  - { mechanism: ledger, answer: works, proof: "test:tests/refund.py" }
exit: refunds post
---
# Drifted
