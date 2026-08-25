# Expand the language adapters

Today `sf` parses Python, TypeScript/TSX, Go, Rust and Ruby. Each new language
is a tree-sitter grammar in `src/lang.rs` — the node kinds that open a
function, the node kinds that branch, the boolean operators — plus one query
per L0 rule that has something to say about it.

**Ruby is done.** See [the ruby language adapter](ruby-language-adapter.md)
for the queries added, the two rules whose Ruby form was judged dishonest and
deliberately left without one, and a real grammar bug the work exposed in the
complexity engine itself, not in this rule's judgment calls.

Java, C# and PHP are the next obvious candidates. Kotlin and Swift matter for
anyone whose product is a mobile app.

A rule with no query for a language simply does not apply to it. That is the
correct outcome and not a gap to paper over: `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`
means nothing in a language whose data layer looks nothing like a session
object, and inventing a query so the coverage table looks full is how a rule
starts producing findings nobody believes.

**Exit condition:** a repository written in the new language runs `sf init`,
`sf verify` reports every enabled rule firing on its fixture, and `sf check`
produces at least one finding that a maintainer of that language agrees is
real — not merely a clean run, which is also what a rule that matches nothing
produces.
