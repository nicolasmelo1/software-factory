// The violation and the accepted form together: the literal assignment
// moves nothing on Windows; the platform-derived one names its own.
process.env.HOME = scratch;
const home = process.platform === "win32" ? "USERPROFILE" : "HOME";
process.env[home] = scratch;
// The accepted restore beside them: an assertion failure before the
// finally still unsets the variable.
try {
  process.env[home] = scratch;
} finally {
  process.env[home] = prev;
}
