---
sources:
  - { id: S1, kind: decision }
touches: ["src/ledger/**", "src/sessions/guest.py"]
criteria:
  - { id: A1, statement: a refund posts to the ledger, proof: "test:tests/refund.py", sources: [S1] }
crosses:
  - { mechanism: ledger, answer: works }
exit: refunds post
---
# Uncrossed
