# Adopting a repository with history

A codebase with years behind it breaks rules you want to enforce from now on.
Freezing what is already there, with a date, is what makes the first day
survivable. Which layers to turn on, and in what order, is the other half of
the same question.

## In which order the layers earn their keep

**Day one: L1, L4, L5.** This is `sf init`'s default, and it is a deliberate
recommendation rather than a shortcut. L1 costs an hour. L4 costs three
markdown files. L5 is what makes either of them mean anything.

**L0 after the third occurrence of a pattern.** Cementing a shape you have seen
twice is how you cement the wrong one. Wait until the repetition tells you what
the shape actually is.

**L6 as soon as you have CI** — `sf init --layer L1,L4,L5,L6` writes the steps
for you. It is the cheapest large win here: the tools already exist and are
better than anything this could contain, and what actually rots is whether they
are still wired in.

**L2's guardrail lock immediately, the rest when a second surface derives from
a first** — a generated client, a schema and its migrations, a design token and
its stylesheet.

**L0 after the third occurrence of a pattern.** Cementing a shape you have seen
twice is how you cement the wrong one. Wait until the repetition tells you what
the shape actually is.

**L3 when there is a customer-visible flow worth proving.** It is the most
valuable layer and the most expensive one; it earns its cost only once
something real can break.

---

## Adopting rules a repository already breaks

```sh
sf ratchet --months 6
```

Every existing violation is frozen by a content-stable key. New ones fail
immediately. Each frozen set carries a `review_by` date, and
`L2.NO_PERMANENT_EXCEPTION` fails the build once it passes — the date does not
force the fix, it forces the conversation, on a day someone chose in advance
rather than never.

Keys are derived from content, not line numbers, so moving code around does not
silently un-freeze the ratchet — and adding a key by hand to silence a new
violation is a visible line in a reviewed diff.

Re-seeding recomputes the frozen keys, never the deadline: a date already
accepted survives, and a newly frozen violation does not reset the clock on the
debt beside it. This matters because `sf ratchet` is part of the prescribed
order after any guardrail change — a run that stamped today + `N` months on
every entry would push every deadline out on every unrelated change, which
`L2.POLICY_ONLY_TIGHTENS` then rejects. Renewing a date that has genuinely
expired is a deliberate edit to the file, with the reasoning in the pull
request, which is the conversation the date exists to force.
