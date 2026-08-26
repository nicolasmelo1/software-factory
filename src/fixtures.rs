//! Mutation fixtures: the smallest repository that violates each rule.
//!
//! These are the reason to trust anything else in this tool. A rule with a
//! typo in its query, a glob that matches nothing, or a scope that excludes
//! the source tree passes silently and looks exactly like a rule that works.
//! `sf verify` runs every enabled rule against its fixture and fails if the
//! rule does not fire.

pub struct Fixture {
    pub rule: &'static str,
    /// Extra policy for the fixture's own mini-repo, merged under `rules:`.
    pub policy_extra: &'static str,
    /// Other rules the fixture needs switched on to be a coherent repository —
    /// the meta rules are about other rules, so they need one to be about.
    pub extra_rules: &'static str,
    pub files: &'static [(&'static str, &'static str)],
}

pub const FIXTURES: &[Fixture] = &[
    Fixture {
        rule: "L0.EXCEPTIONS_HAVE_ONE_HOME",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "src/orders/service.py",
                "class OrderRejectedError(Exception):\n    \"\"\"Defined in a service instead of the domain's errors module.\"\"\"\n",
            ),
            (
                "src/orders/service.ts",
                "// Defined in a service instead of the domain's errors module.\nexport class OrderRejectedError extends Error {}\n",
            ),
            (
                "src/orders/service.go",
                "package orders\n\n// Defined in a service instead of the domain's errors module.\ntype OrderRejectedError struct {\n\tReason string\n}\n",
            ),
            (
                "src/orders/service.rs",
                "// Defined in a service instead of the domain's errors module.\npub enum OrderRejectedError {\n    OutOfStock,\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L0.PERSISTENCE_STAYS_IN_REPOSITORIES",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "src/orders/controllers/get_order.py",
                "def get_order(order_id, db):\n    return db.execute(\"select * from orders where id = %s\", order_id)\n",
            ),
            (
                "src/orders/controllers/get_order.ts",
                "export async function getOrder(orderId: string) {\n  return db.query(\"select * from orders where id = $1\", [orderId]);\n}\n",
            ),
            (
                "src/orders/controllers/get_order.go",
                "package controllers\n\nfunc GetOrder(orderID string) (*Order, error) {\n\treturn db.Query(\"select * from orders where id = $1\", orderID)\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L0.ONE_ENTRYPOINT_PER_FILE",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "src/orders/controllers/orders.py",
                "@router.get(\"/orders\")\ndef list_orders():\n    ...\n\n\n@router.post(\"/orders\")\ndef create_order():\n    ...\n",
            ),
            (
                "src/orders/controllers/orders.ts",
                "router.get(\"/orders\", listOrders);\nrouter.post(\"/orders\", createOrder);\n",
            ),
            (
                "src/orders/handlers/orders.go",
                "package handlers\n\nfunc Register(r *gin.Engine) {\n\tr.GET(\"/orders\", listOrders)\n\tr.POST(\"/orders\", createOrder)\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L0.NO_CROSS_LAYER_IMPORT",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "src/app/main.py",
                "from billing._internal.rates import compute\n\n\ndef price(order):\n    return compute(order)\n",
            ),
            (
                "src/app/main.ts",
                "import { compute } from \"../billing/_internal/rates\";\n\nexport const price = (order: Order) => compute(order);\n",
            ),
            (
                "src/app/main.go",
                "package app\n\nimport \"example.com/billing/internal/rates\"\n\nfunc Price(o Order) int {\n\treturn rates.Compute(o)\n}\n",
            ),
            (
                "src/app/main.rs",
                "use billing::internal::rates::compute;\n\npub fn price(order: &Order) -> u64 {\n    compute(order)\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L1.COMPLEXITY_CEILING",
        policy_extra: "        max: 4\n",
        extra_rules: "",
        files: &[(
            "src/pricing.py",
            "def price(order):\n    total = 0\n    if order.a:\n        total += 1\n    if order.b:\n        total += 1\n    if order.c:\n        total += 1\n    if order.d:\n        total += 1\n    if order.e:\n        total += 1\n    return total\n",
        )],
    },
    Fixture {
        rule: "L1.NO_BLANKET_SUPPRESSION",
        policy_extra: "",
        extra_rules: "",
        files: &[("src/legacy.py", "import os  # noqa\n")],
    },
    Fixture {
        rule: "L1.SKIPPED_TESTS_STATE_A_REASON",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "tests/test_billing.py",
                "@pytest.mark.skip()\ndef test_refund_is_idempotent():\n    ...\n",
            ),
            // Two skips on purpose: the first is the violation, the second is
            // the accepted form. If the `unless` query ever stops matching,
            // the second one starts producing a finding here — which is the
            // only way a fixture can be about a rule staying quiet.
            (
                "tests/billing.test.ts",
                "describe(\"refunds\", () => {\n  it.skip(\"is idempotent\", () => {});\n\n  // Flaky against the sandbox gateway; back when TICKET-4711 lands.\n  it.skip(\"settles twice\", () => {});\n});\n",
            ),
            (
                "tests/billing_test.go",
                "package billing\n\nimport \"testing\"\n\nfunc TestRefundIsIdempotent(t *testing.T) {\n\tt.Skip()\n}\n",
            ),
            (
                "tests/billing.rs",
                "#[test]\n#[ignore]\nfn refund_is_idempotent() {\n    assert!(true);\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L1.NO_UNTYPED_ESCAPE_HATCH",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            "src/payload.py",
            "from typing import Any\n\n\ndef handle(event: dict) -> Any:\n    return event\n",
        )],
    },
    Fixture {
        rule: "L2.GENERATED_FILES_ARE_LOCKED",
        policy_extra: "        scope: [\"generated/**\"]\n        lock_file: \".software-factory/locks/generated.lock.json\"\n",
        extra_rules: "",
        files: &[
            ("generated/schema.json", "{\"version\": 2}\n"),
            (
                ".software-factory/locks/generated.lock.json",
                "{\n  \"schema_version\": 1,\n  \"files\": {\n    \"generated/schema.json\": \"0000000000000000000000000000000000000000000000000000000000000000\"\n  }\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L2.DEPENDENCIES_CHANGE_DELIBERATELY",
        policy_extra: "        scope: [\"package.json\"]\n        lock_file: \".software-factory/locks/dependencies.lock.json\"\n",
        extra_rules: "",
        files: &[
            ("package.json", "{\n  \"dependencies\": {\n    \"left-pad\": \"^1.3.0\"\n  }\n}\n"),
            (
                ".software-factory/locks/dependencies.lock.json",
                "{\n  \"schema_version\": 1,\n  \"files\": {}\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L2.DERIVED_ARTIFACTS_MATCH_THEIR_SOURCE",
        // Fires either way: without --allow-commands because a check that did
        // not run is not a check that passed, and with it because the command
        // fails. `sf verify --allow-commands` exercises the second path.
        policy_extra: "        run: \"exit 1\"\n",
        extra_rules: "",
        files: &[("generated/schema.json", "{\"version\": 1}\n")],
    },
    Fixture {
        rule: "L2.NO_PERMANENT_EXCEPTION",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            ".software-factory/ratchet.yaml",
            "version: 1\nrules:\n  L1.NO_BLANKET_SUPPRESSION:\n    review_by: '2020-01-01'\n    allow:\n      - src/legacy.py:deadbeefdead\n",
        )],
    },
    Fixture {
        rule: "L3.GATE_HAS_FRESH_EVIDENCE",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            "src/checkout/charge.py",
            "def charge(order):\n    \"\"\"Inside an activation path, with no evidence sealed for the gate.\"\"\"\n",
        )],
    },
    Fixture {
        rule: "L4.DOC_LINKS_RESOLVE",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            "docs/architecture.md",
            "# Architecture\n\nSee [the pricing module](../src/pricing/README.md).\n",
        )],
    },
    Fixture {
        rule: "L4.ROOT_FILES_ARE_DECLARED",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (".allowed-root-files", "README.md\n"),
            ("NOTES.md", "# Notes\n\nScratch context that should have been a plan or a PR body.\n"),
            ("README.md", "# Fixture\n"),
        ],
    },
    Fixture {
        rule: "L4.EVERY_RULE_HAS_A_WHY",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            "docs/rules.md",
            "# Rules\n\nThis repository enforces L9.NOT_A_REAL_RULE, which does not exist.\n",
        )],
    },
    Fixture {
        rule: "L4.PLAN_DECLARES_EXIT_CONDITION",
        policy_extra: "",
        extra_rules: "",
        files: &[
            ("plans/next-steps.md", "# Next steps\n\nNothing ordered yet.\n"),
            ("plans/rewrite-checkout.md", "# Rewrite checkout\n\nWe will rewrite checkout.\n"),
        ],
    },
    Fixture {
        rule: "L4.PLAN_CRITERION_NAMES_ITS_CHECK",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            "plans/rewrite-checkout.md",
            "# Rewrite checkout\n\nExit condition: the new checkout serves live traffic.\n\n\
             ## Acceptance criteria\n\n\
             - [ ] A guest can complete a purchase without an account.\n\
             - [ ] Refunds reconcile against the ledger.\n      (proof: test:tests/test_refunds.py)\n",
        )],
    },
    Fixture {
        rule: "L4.CLAIM_CITES_ITS_EVIDENCE",
        policy_extra: "",
        extra_rules: "",
        files: &[("docs/landing.md", A_PAGE_THAT_PROMISES)],
    },
    Fixture {
        rule: "L4.RULE_PROSE_NAMES_A_REAL_COMMAND",
        policy_extra: "",
        // The rule is about other rules' prose, so the fixture needs one to be
        // about: a local rule whose `fix` sends the reader to a subcommand that
        // never existed. This is the exact drift that produced the rule.
        extra_rules: "  L4.LOCAL_RULE_WITH_A_DEAD_COMMAND:\n    enabled: true\n",
        files: &[(".software-factory/rules/local-rule.yaml", RULE_NAMING_A_DEAD_COMMAND)],
    },
    Fixture {
        rule: "L3.GATE_COVERS_THE_PLAN",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            "plans/rewrite-checkout.md",
            "# Rewrite checkout\n\nExit condition: the new checkout serves live traffic.\n\n\
             ## Acceptance criteria\n\n\
             - [ ] A guest can complete a purchase without an account.\n      \
             (proof: assertion:api.guest_checkout_completed)\n",
        )],
    },
    Fixture {
        rule: "L5.EVERY_CHECK_HAS_A_MUTATION_TEST",
        policy_extra: "",
        // A second rule with no fixture of its own is what this must notice.
        extra_rules: "  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n",
        files: &[("src/app.py", "print('a repo enabling rules with nothing proving they fire')\n")],
    },
    Fixture {
        rule: "L2.FACTORY_CONFIG_IS_LOCKED",
        policy_extra: "",
        extra_rules: "",
        files: &[(
            ".software-factory/locks/factory.lock.json",
            "{\n  \"schema_version\": 1,\n  \"files\": {\n    \".software-factory/policy.yaml\": \"0000000000000000000000000000000000000000000000000000000000000000\"\n  }\n}\n",
        )],
    },
    Fixture {
        rule: "L2.POLICY_ONLY_TIGHTENS",
        policy_extra: "        baseline: \"baseline\"\n",
        extra_rules: "",
        files: &[(
            "baseline/.software-factory/policy.yaml",
            "version: 1\nproject:\n  name: baseline\n  languages: [python]\nrules:\n  L2.POLICY_ONLY_TIGHTENS:\n    enabled: true\n  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n",
        )],
    },
    Fixture {
        rule: "L6.DEPENDENCY_VULNERABILITIES_ARE_SCANNED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.SECRETS_ARE_SCANNED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.WORKFLOWS_ARE_SCANNED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.INSECURE_PATTERNS_ARE_SCANNED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.DEAD_CODE_IS_DETECTED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.DATA_RACES_ARE_DETECTED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.PERFORMANCE_REGRESSION_IS_GUARDED",
        policy_extra: "",
        extra_rules: "",
        files: &[(".github/workflows/ci.yml", CI_WITHOUT_HAZARD_TOOLS)],
    },
    Fixture {
        rule: "L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "src/cache.py",
                "def refresh(lock, url):\n    with lock:\n        # The network call runs with every other thread queued behind it.\n        payload = requests.get(url).json()\n        _CACHE.update(payload)\n",
            ),
            (
                "src/cache.go",
                "package cache\n\nfunc Refresh(mu *sync.Mutex) {\n\tmu.Lock()\n\tdefer mu.Unlock()\n\t// Every other goroutine queues behind this sleep.\n\ttime.Sleep(2 * time.Second)\n}\n",
            ),
            (
                "src/cache.rs",
                "pub async fn refresh(state: &Mutex<Cache>, url: &str) {\n    let mut guard = state.lock().expect(\"poisoned\");\n    // Awaiting while holding a synchronous guard: the continuation can be\n    // scheduled onto a thread that then blocks on this same lock.\n    let payload = fetch(url).await;\n    guard.insert(payload);\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L6.ONE_LOCK_AT_A_TIME",
        policy_extra: "",
        extra_rules: "",
        files: &[
            (
                "src/transfer.py",
                "def transfer(source_lock, target_lock, amount):\n    with source_lock:\n        debit(amount)\n        with target_lock:\n            credit(amount)\n",
            ),
            (
                "src/transfer.go",
                "package bank\n\nfunc Transfer(from *Account, to *Account, amount int) {\n\tfrom.mu.Lock()\n\tdefer from.mu.Unlock()\n\tto.mu.Lock()\n\tdefer to.mu.Unlock()\n\tfrom.balance -= amount\n\tto.balance += amount\n}\n",
            ),
            (
                "src/transfer.rs",
                "pub fn transfer(from: &Mutex<Account>, to: &Mutex<Account>, amount: u64) {\n    let mut source = from.lock().expect(\"poisoned\");\n    let mut target = to.lock().expect(\"poisoned\");\n    source.balance -= amount;\n    target.balance += amount;\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L5.NO_INERT_RULE",
        policy_extra: "",
        // A lock switched on over nothing: it passes every run and reads in a
        // report exactly like a lock that is protecting something.
        extra_rules: "  L2.GENERATED_FILES_ARE_LOCKED:\n    enabled: true\n    options:\n      scope: []\n",
        files: &[("src/app.py", "print('a repository with a lock that locks nothing')\n")],
    },
];

/// A repo-local rule that reads perfectly well and tells you to run something
/// this binary has never had. Only the prose is wrong, which is what makes it
/// the mutation: nothing about the check it configures is broken.
const RULE_NAMING_A_DEAD_COMMAND: &str = "\
id: L4.LOCAL_RULE_WITH_A_DEAD_COMMAND\n\
layer: L4\n\
title: A local rule whose fix names a command that does not exist\n\
severity: low\n\
statement: Nothing in this fixture violates this rule; its prose is the mutation.\n\
why: A rule needs a reason, and this one exists to carry a dead command in its fix.\n\
fix: Regenerate the manifest with `sf evidence record`, then commit it.\n\
ratchet: allowlist\n\
check:\n  kind: text_pattern\n\
defaults:\n  \
scope: [\"src/**\"]\n  \
forbidden:\n    \
- regex: \"a shape this fixture never contains\"\n      \
message: \"Unreachable: this rule is here for its prose, not its pattern.\"\n";

/// Two promises and no proof behind either: the first names a gate this
/// policy does not declare, the second names nothing at all. The third is
/// fenced, so it is the page showing the form rather than promising anything,
/// and it must stay quiet or the rule cannot be documented anywhere it scans.
const A_PAGE_THAT_PROMISES: &str = "\
# What this does\n\n\
<!-- claim: IMPORT_50K_UNDER_60S proven-by: bulk-import -->\n\
Import fifty thousand rows in under a minute.\n\n\
<!-- claim: SEARCH_IS_INSTANT -->\n\
Search feels instant on the largest workspace anyone has.\n\n\
Mark a promise like this:\n\n\
```\n\
<!-- claim: SOME_ID proven-by: some-gate -->\n\
```\n";

/// A believable CI file that tests and lints and hunts none of the hazards.
const CI_WITHOUT_HAZARD_TOOLS: &str = "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: pytest\n      - run: npm test\n      - run: go test ./...\n      - run: cargo test\n";


/// The mini-policy a fixture runs under: the target rule, plus whatever the
/// fixture needs to be a coherent repository.
pub fn fixture_policy(fixture: &Fixture) -> String {
    let gates = match fixture.rule {
        "L3.GATE_HAS_FRESH_EVIDENCE" => {
            "gates:\n  checkout:\n    activation: [\"src/checkout/**\"]\n    evidence: \"evidence/checkout.json\"\n"
        }
        // A gate that names its plan and requires an assertion the plan does
        // not cite. The criterion cites a different one, which is the hole.
        "L3.GATE_COVERS_THE_PLAN" => {
            "gates:\n  checkout:\n    activation: [\"src/checkout/**\"]\n    evidence: \"evidence/checkout.json\"\n    plan: \"plans/rewrite-checkout.md\"\n    required_assertions: [\"api.ledger_balanced\"]\n"
        }
        _ => "gates: {}\n",
    };
    let extra_rule = fixture.extra_rules;
    format!(
        "# Generated mutation fixture for {rule}. It is supposed to fail.\n\
         version: 1\n\
         project:\n  \
           name: mutation-{rule}\n  \
           languages: [python, typescript, go, rust]\n\
         docs:\n  \
           scan: [\"docs/**/*.md\"]\n\
         {gates}\
         rules:\n\
         {extra_rule}\
         \x20 {rule}:\n    enabled: true\n{options}",
        rule = fixture.rule,
        options = if fixture.policy_extra.is_empty() {
            String::new()
        } else {
            format!("    options:\n{}", fixture.policy_extra)
        }
    )
}

/// The policy a template-generated rule's fixture runs under.
pub fn minimal_policy(rule_id: &str) -> String {
    format!(
        "# Generated mutation fixture for {rule_id}. It is supposed to fail.\n\
         version: 1\n\
         project:\n  name: mutation\n  languages: [python, typescript, go, rust]\n\
         rules:\n  {rule_id}:\n    enabled: true\n"
    )
}

pub fn for_rule(rule: &str) -> Option<&'static Fixture> {
    FIXTURES.iter().find(|f| f.rule == rule)
}
