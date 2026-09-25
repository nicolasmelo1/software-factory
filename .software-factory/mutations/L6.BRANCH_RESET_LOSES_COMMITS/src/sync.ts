// The violation and the accepted form together: the second line names the
// rev-parse check that proves the tip reachable — the `unless` that makes a
// deliberate move expressible.
await git(repo, "checkout", "-B", branch, base);
await git(repo, "checkout", "-B", branch, "origin/" + branch); // rev-parse verified
