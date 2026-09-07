# Hunting defect classes

`sf` does not reimplement a vulnerability database, a secret scanner or a race
detector. Those tools exist, they are better than anything that could live
here, and they differ per language. What is missing in most repositories is not
the tool — it is the guarantee that the tool is *still wired in*. So a rule
names a concern, and the check asserts that something covering it actually runs
in your CI or task runner.

<!-- sf:generated hazard-tools -->
| Concern | go | python | ruby | rust | typescript |
| :-- | :-- | :-- | :-- | :-- | :-- |
| Concurrent code is exercised under a race detector | -race, go test -race | no tool listed | no tool listed | thread-sanitizer, -Zsanitizer=thread, loom | no tool listed |
| Something detects code nothing reaches | staticcheck, deadcode, unused | vulture, deadcode | no tool listed | cargo udeps, cargo-udeps, dead_code | ts-prune, knip, depcheck |
| Something audits dependencies for known vulnerabilities | govulncheck, osv-scanner, nancy, snyk | pip-audit, osv-scanner, safety, snyk | bundler-audit, bundle-audit, osv-scanner | cargo audit, cargo-audit, cargo-deny, osv-scanner | npm audit, pnpm audit, yarn audit, osv-scanner, snyk |
| Something scans the code for known-insecure patterns | gosec, semgrep, staticcheck, codeql | bandit, semgrep, codeql | brakeman, semgrep, codeql | cargo-geiger, cargo clippy, clippy, semgrep | semgrep, eslint-plugin-security, codeql |
| Something would notice the code getting slower | go test -bench, benchstat, -bench | pytest-benchmark, asv, richbench | no tool listed | criterion, cargo bench, divan | vitest bench, benchmark.js, tinybench, hyperfine |
| Something scans for committed secrets | detect-secrets, gitleaks, trufflehog | detect-secrets, gitleaks, trufflehog | detect-secrets, gitleaks, trufflehog | detect-secrets, gitleaks, trufflehog | detect-secrets, gitleaks, trufflehog |
| Something scans the CI workflows themselves | zizmor, poutine, octoscan | zizmor, poutine, octoscan | zizmor, poutine, octoscan | zizmor, poutine, octoscan | zizmor, poutine, octoscan |

<!-- sf:end hazard-tools -->

`sf init` writes these steps into the generated workflow for the languages you
selected. A concern with no listed tool for a language is not a violation —
that is a statement about the ecosystem, not about your repository.

## What static analysis cannot do

**Nothing decides whether a program deadlocks.** It is undecidable in general,
and a tool claiming otherwise teaches you to trust it wrongly. What *is*
decidable is the shape that causes the deadlocks and starvation people actually
ship, and two rules enforce exactly that:

- **`L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`** — a network call, a sleep, a
  subprocess or an `await` inside a region that holds a lock. Holding a lock
  across something slow turns mutual exclusion into a queue, which is
  starvation; awaiting while holding a synchronous lock can park the
  continuation on a thread that then blocks on that same lock, and nothing
  moves again.
- **`L6.ONE_LOCK_AT_A_TIME`** — a second lock acquired while the first is held.
  It cannot tell a correctly ordered pair from a dangerous one; ordering is a
  global property and the rule sees one function. It makes the second
  acquisition *visible*, which is the part that is otherwise invisible. If the
  pair is genuinely correct, freeze it in the ratchet with the ordering written
  beside it — now the ordering is documented, which is the only thing that ever
  prevents the inversion.

Data races get the same honesty: no static check finds them, so the rule
requires the dynamic detector to run instead.
