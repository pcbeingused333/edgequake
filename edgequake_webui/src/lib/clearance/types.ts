export interface ClearanceSettings {
  enforcement: boolean;
  policy_epoch: number;
  steward: boolean;
  rank: string;
  can_manage: boolean;
}

export interface ClearanceTag {
  id: string;
  kind: string;
  key: string;
  value: string;
  rank: number | null;
}

/** One saved rule, as the API returns it. The editor maps it back to a draft. */
export interface RuleView {
  effect: "allow" | "deny" | string;
  match_kind: "classification_at_most" | "any_of" | "untagged" | string;
  rank: number | null;
  tag_ids: string[];
}

export interface SecurityRoleView {
  id: string;
  name: string;
  steward: boolean;
  rules: RuleView[];
}

export interface MemberRolesResponse {
  role_ids: string[];
}

/** Classification words as stored on a document. */
export type ClassificationWord =
  | "public"
  | "internal"
  | "confidential"
  | "restricted"
  | "unverified";
