// A fold: the engine hands it a record it has already advanced, so
// record.state here is the state it was moved to, not the state the plan
// was made in. applyPlan itself is the convention, not a violation.
import { applyPlan, Plan } from "@amykit/core";

export function applyOutcomes(record: OncallRecord, outcomes: EffectOutcomes): OncallRecord {
  const next = { ...record };
  if (record.state === "received" && outcomes.attempt) next.lastAttempt = outcomes.attempt;
  return next;
}

export function applyNotePlan(record: OncallRecord, plan: Plan, now: Date): OncallRecord {
  return applyPlan(record, plan, now);
}
