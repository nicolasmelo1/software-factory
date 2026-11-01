---
sources:
  - { id: S1, kind: decision }
  - { id: S2, kind: ticket }
criteria:
  - { id: A1, statement: a guest pays, proof: "test:tests/pay.py" }
  - { id: A2, statement: a refund posts, proof: "test:tests/refund.py", sources: [S1] }
notes: context nobody owns
exit: checkout serves traffic
---
# Unlinked
