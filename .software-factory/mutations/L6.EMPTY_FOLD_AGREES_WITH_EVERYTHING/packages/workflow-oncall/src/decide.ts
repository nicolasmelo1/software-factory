// The violation and the accepted guard together: the second fold
// proves the collection is non-empty before it asks every() to decide.
export const approved = (approvals: string[]) =>
  approvals.every((a) => a === "yes");
export const reviewed = (approvals: string[]) =>
  approvals.length > 0 && approvals.every((a) => a === "yes");
