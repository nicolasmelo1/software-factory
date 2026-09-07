# Documentation generation

Documentation is hand-written everywhere except marker blocks. A block is
replaced only between its matching markers:

```markdown
&lt;!-- sf:generated rules-summary --&gt;
generated content
&lt;!-- sf:end rules-summary --&gt;
```

The supported blocks are `rules-summary`, `command-surface`, and
`language-coverage`. Their values are read from the catalog, policy, ratchet,
and the clap definition in `src/main.rs`; they contain no timestamps or other
run-dependent values. Unknown blocks and blocks with no placement are errors.

Run `sf docs` to update documentation. Run `sf docs --check` to check without
writing; it lists every page that would change and prints the fixing command.
The existing generated section of `docs/rules.md` remains owned by `sf docs`
and preserves all prose above its first `## L` heading.