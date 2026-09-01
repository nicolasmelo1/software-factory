#!/bin/sh
# The `adoption` gate's scenario, as a repeatable run.
#
# Usage: adoption-scenario.sh <path-to-a-typescript-repo> [report-path]
#
# Copies a repository nobody tuned this tool for into a scratch checkout, walks
# the README quickstart against it, then writes one new violation and checks
# again. Emits the report `.software-factory/evidence/adoption.json` cites.
#
# A harness, not the actor. It runs the commands a person adopting the tool
# would type; who invokes it is what the manifest's `actor` records, and
# `L3.GATE_HAS_FRESH_EVIDENCE` refuses a manifest that credits the run to the
# harness itself.
set -eu

corpus=${1:?usage: adoption-scenario.sh <path-to-a-typescript-repo> [report-path]}
report=${2:-.software-factory/evidence/adoption-run.json}
sf=$(cd "$(dirname "$0")/../.." && pwd)/target/release/sf
test -x "$sf" || { echo "build it first: cargo build --release" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
checkout="$work/adopter"

corpus_sha=$(git -C "$corpus" rev-parse HEAD)
rsync -a --exclude .git --exclude node_modules --exclude dist --exclude build \
      "$corpus/" "$checkout/"
git -C "$checkout" init -q
git -C "$checkout" add -A
git -C "$checkout" -c user.email=scenario@local -c user.name=scenario \
    commit -qm "the corpus as an adopter first sees it"
tracked=$(git -C "$checkout" ls-files | wc -l | tr -d ' ')

# 1. Setup. The README quickstart, with the tool picking its own default layers.
"$sf" init --root "$checkout" --name adopter --language typescript >"$work/init.log" 2>&1
scaffolded=passed
for f in .software-factory/policy.yaml docs/rules.md \
         .github/workflows/software-factory.yml .githooks/pre-commit; do
    test -f "$checkout/$f" || scaffolded=failed
done
# A policy that parses is the difference between a file and a policy.
"$sf" explain L1.COMPLEXITY_CEILING --root "$checkout" >/dev/null 2>&1 || scaffolded=failed
rules=$(grep -c 'enabled: true' "$checkout/.software-factory/policy.yaml" || true)
frozen=$(grep -cE '^\s+- ' "$checkout/.software-factory/ratchet.yaml" || true)

# 2. Every rule that init enabled is proven to fire on its own fixture.
if "$sf" verify --root "$checkout" >"$work/verify.log" 2>&1; then
    verified=passed
else
    verified=failed
fi
proven=$(sed -n 's/^\([0-9]*\/[0-9]*\) enabled rules proven to fire$/\1/p' "$work/verify.log")

# 3. Green on the baseline init froze. Adoption must not hand anybody a red build.
if "$sf" check --root "$checkout" >"$work/check-baseline.log" 2>&1; then
    baseline=passed
else
    baseline=failed
fi

# 4. The half that matters: the first new violation after adoption is refused.
probe=packages/core/src/adoption-probe.ts
mkdir -p "$(dirname "$checkout/$probe")"
{
    echo '// A new file, written after adoption, the way a hurried change arrives.'
    echo 'export function resolveOptions(input: any) {'
    echo '  let out = 0;'
    for branch in a b c d e f g h i j k l m; do
        echo "  if (input.$branch) out += 1;"
    done
    echo '  return out;'
    echo '}'
} >"$checkout/$probe"
if "$sf" check --root "$checkout" >"$work/check-probe.log" 2>&1; then
    refused=failed   # green after a new violation is the failure this proves against
else
    refused=passed
fi
grep -q "$probe" "$work/check-probe.log" || refused=failed

cat >"$report" <<JSON
{
  "scenario": "adoption",
  "status": "$( [ "$scaffolded$verified$baseline$refused" = "passedpassedpassedpassed" ] && echo passed || echo failed )",
  "goal": "I maintain a TypeScript framework with a few years of history in it. Install the software factory, set it up the way the README says to, and tell me whether it will stop the next bad change I write without drowning me in the mess that is already there.",
  "corpus": {
    "repository": "$(basename "$corpus")",
    "commit": "$corpus_sha",
    "tracked_files": $tracked
  },
  "sf": "$("$sf" --version)",
  "observed": {
    "rules_enabled_by_init": $rules,
    "violations_frozen_at_init": $frozen,
    "verify": "$proven enabled rules proven to fire",
    "check_on_baseline": "$(tail -1 "$work/check-baseline.log")",
    "check_after_one_new_violation": "$(grep -E 'findings across' "$work/check-probe.log" | tail -1)"
  },
  "assertions": [
    { "type": "cli.init_scaffolds_a_policy", "status": "$scaffolded" },
    { "type": "cli.verify_is_green_after_init", "status": "$verified" },
    { "type": "cli.check_is_green_on_the_frozen_baseline", "status": "$baseline" },
    { "type": "cli.new_violation_fails_the_check", "status": "$refused" }
  ]
}
JSON
echo "wrote $report"
