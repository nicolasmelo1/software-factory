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
    pub files: &'static [(&'static str, &'static str)],
}

pub const FIXTURES: &[Fixture] = &[
    Fixture {
        rule: "L0.EXCEPTIONS_HAVE_ONE_HOME",
        policy_extra: "",
        files: &[(
            "src/orders/service.py",
            "class OrderRejectedError(Exception):\n    \"\"\"Defined in a service instead of the domain's errors module.\"\"\"\n",
        )],
    },
    Fixture {
        rule: "L0.PERSISTENCE_STAYS_IN_REPOSITORIES",
        policy_extra: "",
        files: &[(
            "src/orders/controllers/get_order.py",
            "def get_order(order_id, db):\n    return db.execute(\"select * from orders where id = %s\", order_id)\n",
        )],
    },
    Fixture {
        rule: "L0.ONE_ENTRYPOINT_PER_FILE",
        policy_extra: "",
        files: &[(
            "src/orders/controllers/orders.py",
            "@router.get(\"/orders\")\ndef list_orders():\n    ...\n\n\n@router.post(\"/orders\")\ndef create_order():\n    ...\n",
        )],
    },
    Fixture {
        rule: "L0.NO_CROSS_LAYER_IMPORT",
        policy_extra: "",
        files: &[(
            "src/app/main.py",
            "from billing._internal.rates import compute\n\n\ndef price(order):\n    return compute(order)\n",
        )],
    },
    Fixture {
        rule: "L1.COMPLEXITY_CEILING",
        policy_extra: "        max: 4\n",
        files: &[(
            "src/pricing.py",
            "def price(order):\n    total = 0\n    if order.a:\n        total += 1\n    if order.b:\n        total += 1\n    if order.c:\n        total += 1\n    if order.d:\n        total += 1\n    if order.e:\n        total += 1\n    return total\n",
        )],
    },
    Fixture {
        rule: "L1.NO_BLANKET_SUPPRESSION",
        policy_extra: "",
        files: &[("src/legacy.py", "import os  # noqa\n")],
    },
    Fixture {
        rule: "L1.NO_UNTYPED_ESCAPE_HATCH",
        policy_extra: "",
        files: &[(
            "src/payload.py",
            "from typing import Any\n\n\ndef handle(event: dict) -> Any:\n    return event\n",
        )],
    },
    Fixture {
        rule: "L2.GENERATED_FILES_ARE_LOCKED",
        policy_extra: "        scope: [\"generated/**\"]\n        lock_file: \".software-factory/locks/generated.lock.json\"\n",
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
        files: &[
            ("package.json", "{\n  \"dependencies\": {\n    \"left-pad\": \"^1.3.0\"\n  }\n}\n"),
            (
                ".software-factory/locks/dependencies.lock.json",
                "{\n  \"schema_version\": 1,\n  \"files\": {}\n}\n",
            ),
        ],
    },
    Fixture {
        rule: "L2.NO_PERMANENT_EXCEPTION",
        policy_extra: "",
        files: &[(
            ".software-factory/ratchet.yaml",
            "version: 1\nrules:\n  L1.NO_BLANKET_SUPPRESSION:\n    review_by: '2020-01-01'\n    allow:\n      - src/legacy.py:deadbeefdead\n",
        )],
    },
    Fixture {
        rule: "L3.GATE_HAS_FRESH_EVIDENCE",
        policy_extra: "",
        files: &[(
            "src/checkout/charge.py",
            "def charge(order):\n    \"\"\"Inside an activation path, with no evidence sealed for the gate.\"\"\"\n",
        )],
    },
    Fixture {
        rule: "L4.DOC_LINKS_RESOLVE",
        policy_extra: "",
        files: &[(
            "docs/architecture.md",
            "# Architecture\n\nSee [the pricing module](../src/pricing/README.md).\n",
        )],
    },
    Fixture {
        rule: "L4.ROOT_FILES_ARE_DECLARED",
        policy_extra: "",
        files: &[
            (".allowed-root-files", "README.md\n"),
            ("NOTES.md", "# Notes\n\nScratch context that should have been a plan or a PR body.\n"),
            ("README.md", "# Fixture\n"),
        ],
    },
    Fixture {
        rule: "L4.EVERY_RULE_HAS_A_WHY",
        policy_extra: "",
        files: &[(
            "docs/rules.md",
            "# Rules\n\nThis repository enforces L9.NOT_A_REAL_RULE, which does not exist.\n",
        )],
    },
    Fixture {
        rule: "L4.PLAN_DECLARES_EXIT_CONDITION",
        policy_extra: "",
        files: &[
            ("plans/next-steps.md", "# Next steps\n\nNothing ordered yet.\n"),
            ("plans/rewrite-checkout.md", "# Rewrite checkout\n\nWe will rewrite checkout.\n"),
        ],
    },
    Fixture {
        rule: "L5.EVERY_CHECK_HAS_A_MUTATION_TEST",
        policy_extra: "",
        files: &[("src/app.py", "print('a repo enabling rules with nothing proving they fire')\n")],
    },
];

/// The mini-policy a fixture runs under: the target rule, plus whatever the
/// fixture needs to be a coherent repository.
pub fn fixture_policy(fixture: &Fixture) -> String {
    let gates = if fixture.rule == "L3.GATE_HAS_FRESH_EVIDENCE" {
        "gates:\n  checkout:\n    activation: [\"src/checkout/**\"]\n    evidence: \"evidence/checkout.json\"\n"
    } else {
        "gates: {}\n"
    };
    // L5 needs a second enabled rule to have nothing proving *it* fires.
    let extra_rule = if fixture.rule == "L5.EVERY_CHECK_HAS_A_MUTATION_TEST" {
        "  L1.NO_BLANKET_SUPPRESSION:\n    enabled: true\n"
    } else {
        ""
    };
    format!(
        "# Generated mutation fixture for {rule}. It is supposed to fail.\n\
         version: 1\n\
         project:\n  \
           name: mutation-{rule}\n  \
           languages: [python, typescript, go]\n\
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

pub fn for_rule(rule: &str) -> Option<&'static Fixture> {
    FIXTURES.iter().find(|f| f.rule == rule)
}
