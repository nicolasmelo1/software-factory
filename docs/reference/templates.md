# Rule templates

A template is a rule with your own package and directory names left blank. The
interview fills them in, and the result carries a fixture proving it fires
before it is written into your repository.

## What ships

<!-- sf:generated templates-index -->
| Template | Rule it writes | What that rule requires | Filled in with |
| :-- | :-- | :-- | :-- |
| `schemas-live-with-their-handler` | `L0.SCHEMAS_LIVE_WITH_THEIR_HANDLER` | Request and response schemas are defined beside the handler that uses them | nothing; it ships as written |
| `no-fetch-inside-an-effect` | `L1.NO_FETCH_INSIDE_AN_EFFECT` | Data fetching does not happen inside an effect | nothing; it ships as written |
| `global-state-lives-in-one-place` | `L0.GLOBAL_STATE_LIVES_IN_ONE_PLACE` | Global stores are created in one directory | nothing; it ships as written |
| `client-never-imports-the-data-layer` | `L0.CLIENT_NEVER_IMPORTS_THE_DATA_LAYER` | Client code never imports the data layer directly | `client_root_first`, `client_root_globs`, `data_layer_packages`, `data_layer_packages_first`, `data_layer_packages_pattern` |

<!-- sf:end templates-index -->

Placeholders are `@@name@@`, never `${name}`: a fixture contains real source
in several languages, and `${...}` is a template literal in two of them.

A template carries its own fixture block and is validated when filled in, so a
template that would produce an unrunnable rule fails at `sf init` rather than
in somebody's repository.
