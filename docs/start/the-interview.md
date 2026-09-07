# The interview

Bare `sf init` cannot know whether you use repositories or call the ORM from
services, whether errors live per-module or in one file, or which package the
client must never import. So it enables the generic rules and leaves the
structural ones off.

The interview fixes that, and it is deliberately split in two:

- **The conversation belongs to the agent.** It reads your repo, answers what
  the code can answer, and pushes back when you say "layered" and a route
  handler opens a database connection.
- **The mapping does not.** Which rules an answer produces lives in
  `sf interview`, as data. Two agents interviewing the same team land on the
  same policy — otherwise it is just each agent's taste with extra steps.

```sh
sf interview          # the decision tree, and what each answer enforces
sf interview --json   # the same, for an agent conducting it
```

Twelve decisions, walked as a tree in rounds: what the repository is, how it is
organised, which framework, how code reaches the database, where error types
live, what validates the boundary and where those schemas sit, how the client
fetches and stores state, which packages the client may never import, whether
anything shares mutable state across threads, and what is generated rather than
written.

Answers go in a file, and the file is the decision:

```yaml
# .software-factory/answers.yaml
version: 1
answers:
  kind: backend-service
  architecture: hexagonal
  framework: fastapi
  data_access: repositories
  errors_home: per-module
  validation: pydantic
  validation_placement: with-the-handler
  concurrency: shared-state
  generated: "src/generated/**, **/*_pb2.py"
```

```sh
sf init --language python --layer L1,L4,L5,L6 --answers .software-factory/answers.yaml
```

That enables the L0 rules those answers justify (and only those), points them
at the right directories, switches off the ones that cannot mean anything here,
instantiates **repo-specific rules from templates** with your own package names
filled in — each with a fixture proving it fires — and writes
`docs/architecture-decisions.md` recording who decided what.

A real example, on a TypeScript monorepo with a Next.js client:

```
$ sf init --language typescript --layer L1,L4,L5,L6 --answers answers.yaml
  .software-factory/rules/client-never-imports-the-data-layer.yaml
  .software-factory/rules/no-fetch-inside-an-effect.yaml
  docs/architecture-decisions.md
  .software-factory/ratchet.yaml (118 existing violations frozen)

$ sf verify
14/14 enabled rules proven to fire

$ sf check
✓ 14 rules, no findings (118 frozen by the ratchet)
```

The generated `L0.CLIENT_NEVER_IMPORTS_THE_DATA_LAYER` carries that repo's
actual package names in its tree-sitter query and its own `apps/web/**` in the
constraint. It found zero violations — the boundary already held, and now
nothing can quietly break it.

Change an answer and re-run. Do not hand-edit the generated policy, or the
decision record stops describing what is enforced.

---

## Every decision, and what each answer does

Read out of `interview/decisions.yaml`, which is the same tree `sf interview`
prints.

<!-- sf:generated interview-tree -->
### `kind` — What is this repository?

It decides which rule families can mean anything here. Half the catalog is
about an HTTP surface that a CLI does not have.

| Answer | What it does to the policy |
| :-- | :-- |
| `backend-service` | Nothing on its own. |
| `web-client` | switches off `L0.ONE_ENTRYPOINT_PER_FILE`, `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`, `L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`, `L6.ONE_LOCK_AT_A_TIME`, `L6.DATA_RACES_ARE_DETECTED`. |
| `mobile-client` | switches off `L0.ONE_ENTRYPOINT_PER_FILE`, `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`, `L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`, `L6.ONE_LOCK_AT_A_TIME`, `L6.DATA_RACES_ARE_DETECTED`. |
| `cli` | switches off `L0.ONE_ENTRYPOINT_PER_FILE`. |
| `library` | switches off `L0.ONE_ENTRYPOINT_PER_FILE`. |

### `architecture` — How is the code organised, or how do you intend to organise it?

This is the single answer that produces the most rules, because every layered
architecture is a claim about which code may call which — and that claim is
exactly what erodes first under agent-heavy work.

Asked only when `kind` is backend-service or cli or library.

| Answer | What it does to the policy |
| :-- | :-- |
| `layered` | enables `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`, `L0.ONE_ENTRYPOINT_PER_FILE`, `L0.EXCEPTIONS_HAVE_ONE_HOME`; sets options on `L0.ONE_ENTRYPOINT_PER_FILE`, `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`. |
| `ddd` | enables `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`, `L0.EXCEPTIONS_HAVE_ONE_HOME`, `L0.NO_CROSS_LAYER_IMPORT`; sets options on `L0.EXCEPTIONS_HAVE_ONE_HOME`, `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`. |
| `hexagonal` | enables `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`, `L0.NO_CROSS_LAYER_IMPORT`, `L0.EXCEPTIONS_HAVE_ONE_HOME`; sets options on `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`. |
| `modular-monolith` | enables `L0.EXCEPTIONS_HAVE_ONE_HOME`, `L0.ONE_ENTRYPOINT_PER_FILE`; sets options on `L0.ONE_ENTRYPOINT_PER_FILE`. |
| `none-yet` | switches off `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`, `L0.ONE_ENTRYPOINT_PER_FILE`, `L0.NO_CROSS_LAYER_IMPORT`. |

### `framework` — Which HTTP framework?

It decides what a route declaration looks like, which is what the
one-entrypoint-per-file rule has to recognise.

Asked only when `kind` is backend-service.

| Answer | What it does to the policy |
| :-- | :-- |
| `fastapi` | Nothing on its own. |
| `django` | sets options on `L0.ONE_ENTRYPOINT_PER_FILE`. |
| `flask` | Nothing on its own. |
| `express` | Nothing on its own. |
| `nest` | sets options on `L0.ONE_ENTRYPOINT_PER_FILE`. |
| `hono` | Nothing on its own. |
| `gin` | Nothing on its own. |
| `other` | Nothing on its own. |

### `data_access` — How does code reach the database?

Reaching for the session directly is always the shortest diff, so this is the
boundary agents erode fastest. It is also the one whose loss is most expensive
to recover.

Asked only when `kind` is backend-service.

| Answer | What it does to the policy |
| :-- | :-- |
| `repositories` | enables `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`. |
| `orm-in-services` | sets options on `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`. |
| `no-database` | switches off `L0.PERSISTENCE_STAYS_IN_REPOSITORIES`. |

### `errors_home` — Where do error types get defined?

So that "what can this fail with?" is one file read rather than a search
across services, repositories and handlers.

| Answer | What it does to the policy |
| :-- | :-- |
| `per-module` | enables `L0.EXCEPTIONS_HAVE_ONE_HOME`. |
| `single-shared` | enables `L0.EXCEPTIONS_HAVE_ONE_HOME`; sets options on `L0.EXCEPTIONS_HAVE_ONE_HOME`. |
| `no-convention` | switches off `L0.EXCEPTIONS_HAVE_ONE_HOME`. |

### `validation` — What validates data crossing the boundary?

A schema is the only place a boundary is actually described. Where the schemas
live decides whether that description is findable.

| Answer | What it does to the policy |
| :-- | :-- |
| `pydantic` | Nothing on its own. |
| `zod` | Nothing on its own. |
| `go-playground` | Nothing on its own. |
| `none` | Nothing on its own. |

### `validation_placement` — Where do request and response schemas live?

Schemas dumped in a shared types module stop being about any one endpoint, and
the boundary they described becomes unfindable from the handler that owns it.

Asked only when `validation` is pydantic or zod or go-playground.

| Answer | What it does to the policy |
| :-- | :-- |
| `with-the-handler` | writes the template `schemas-live-with-their-handler`. |
| `central-module` | Nothing on its own. |
| `no-convention` | Nothing on its own. |

### `client_data` — How does the client fetch server data?

Fetching inside an effect is the pattern that produces the race conditions,
duplicate requests and stale reads a query cache exists to prevent — and it
is what an agent writes when nothing says otherwise.

Asked only when `kind` is web-client or mobile-client.

| Answer | What it does to the policy |
| :-- | :-- |
| `react-query` | writes the template `no-fetch-inside-an-effect`. |
| `server-components` | writes the template `no-fetch-inside-an-effect`. |
| `manual` | Nothing on its own. |

### `client_state` — Where does global client state live?

A store created inline in a component is invisible to everyone else and
duplicates on re-render. One directory makes the whole of global state
answerable by listing it.

Asked only when `kind` is web-client or mobile-client.

| Answer | What it does to the policy |
| :-- | :-- |
| `single-directory` | writes the template `global-state-lives-in-one-place`. |
| `context-only` | Nothing on its own. |
| `no-convention` | Nothing on its own. |

### `client_root` — Where does the client application live?

The boundary rules need to know which directory is the client half, and a
wrong answer makes them either silent or unbearable.

Asked only when `kind` is web-client or mobile-client.

Free text. Nothing on its own.

### `data_layer_packages` — Which packages must the client never import directly?

In a monorepo the first import of a database client into a component is not a
compile error. It builds, it works in dev, and it either ships credentials
into a browser bundle or drags a driver into the build.

Asked only when `kind` is web-client or mobile-client.

Free text. writes the template `client-never-imports-the-data-layer`.

### `concurrency` — Does this code share mutable state across threads or tasks?

No checker decides whether a program deadlocks. The lock-shape rules and the
race detector are the decidable parts, and they are noise in code that has no
concurrency at all.

Asked only when `kind` is backend-service or cli or library.

| Answer | What it does to the policy |
| :-- | :-- |
| `shared-state` | enables `L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`, `L6.ONE_LOCK_AT_A_TIME`, `L6.DATA_RACES_ARE_DETECTED`. |
| `message-passing` | enables `L6.DATA_RACES_ARE_DETECTED`; switches off `L6.ONE_LOCK_AT_A_TIME`. |
| `single-threaded` | switches off `L6.NO_BLOCKING_CALL_WHILE_HOLDING_A_LOCK`, `L6.ONE_LOCK_AT_A_TIME`, `L6.DATA_RACES_ARE_DETECTED`. |

### `generated` — Which files are generated, vendored or otherwise not hand-written?

Editing a generated file by hand is the smallest possible fix and it silently
forks the artifact from its source. The lock makes that impossible rather than
discouraged.

Free text. enables `L2.GENERATED_FILES_ARE_LOCKED`; points at your own paths `L2.GENERATED_FILES_ARE_LOCKED`.
<!-- sf:end interview-tree -->
