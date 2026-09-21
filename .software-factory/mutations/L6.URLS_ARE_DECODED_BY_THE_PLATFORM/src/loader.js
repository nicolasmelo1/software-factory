// The three ways a hand decode goes wrong, all on lines the platform's
// decoder never appears on.
const decoded = decodeURIComponent(spec) === "file:" ? decodeURIComponent(spec) : spec;
const stripped = spec.replace("file://", "");
const sliced = spec.slice("file://".length);
// The accepted form beside them: the platform's own decoder.
import { fileURLToPath } from "node:url";
const entry = fileURLToPath(new URL(spec));
