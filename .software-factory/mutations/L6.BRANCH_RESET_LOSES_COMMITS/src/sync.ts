// The violation and the accepted form together: the second line's
// fetch-and-verify is the `unless` that makes a deliberate move expressible.
await git(repo, "checkout", "-B", branch, base);
await git(repo, "checkout", "-B", branch, "origin/" + branch); // rev-parse verified
