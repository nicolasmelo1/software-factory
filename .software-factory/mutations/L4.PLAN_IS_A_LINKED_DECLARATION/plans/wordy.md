---
sources:
  - { id: S1, kind: decision }
criteria:
  - { id: A1, statement: a guest who has never signed in before can complete a whole purchase without an account, proof: "test:tests/pay.py", sources: [S1] }
exit: checkout serves traffic
---
# Wordy

First line of context.
Second line of context.
Third line of context.
