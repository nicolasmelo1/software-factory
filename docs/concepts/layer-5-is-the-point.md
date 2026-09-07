# Layer 5 is the whole point

A check with a typo in its query, a glob that matches nothing, or a scope that
excludes the entire source tree passes silently forever — and reads exactly
like a check that works. A green build proves nothing about a rule that never
ran.

So every rule ships with the smallest repository that violates it, and
`sf verify` runs each rule against its own mutation:

```
$ sf verify

✓ L0.EXCEPTIONS_HAVE_ONE_HOME — 1 finding(s): `OrderRejectedError` is defined outside its allowed location
✓ L1.COMPLEXITY_CEILING — 1 finding(s): `price` has 6 independent paths, ceiling is 4
✓ L1.NO_BLANKET_SUPPRESSION — 1 finding(s): Bare `# noqa` disables every rule on the line...
✓ L4.DOC_LINKS_RESOLVE — 1 finding(s): link target `../src/pricing/README.md` does not exist
...
12/12 enabled rules proven to fire
```

The generated pre-commit hook and CI workflow run `sf verify` **before**
`sf check`, because a check that stopped firing is the cheaper failure to find
first.
