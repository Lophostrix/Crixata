/**
 * Crixata Cache Schema & Types
 * Mirror of packages/cache-schema/schema.json
 */

export type Grade = 'A' | 'B' | 'C' | 'D';

export type PolicyType =
  | 'privacy_policy'
  | 'terms_of_service'
  | 'cookie_policy'
  | 'other';

export type GradeSource = 'curated' | 'community' | 'llm';

export interface UserRights {
  /** Whether the user has explicit right to request deletion of personal data */
  can_delete_data: boolean;
  /** Whether the user has explicit right to download/export personal data */
  can_export_data: boolean;
  /** Whether the user has explicit option to opt-out of behavioral tracking or third-party ads */
  can_opt_out_of_tracking: boolean;
}

export interface PolicySummary {
  /** Categories of personal data collected */
  data_collected: string[];
  /** Purposes for which collected data is used */
  data_used_for: string[];
  /** Whether personal data is shared with or sold to third parties */
  shared_with_third_parties: boolean;
  /** Details regarding third-party sharing or categories of recipients */
  third_party_details: string;
  /** Data retention duration or criteria stated in the policy */
  retention_period: string;
  /** User privacy and control rights */
  user_rights: UserRights;
  /** Information about cookies, web beacons, and advertising trackers */
  tracking_and_ads: string;
  /** Whether the policy contains mandatory arbitration or class-action waiver clauses */
  arbitration_or_class_action_waiver: boolean;
  /** Readability and transparency assessment of the policy text */
  policy_clarity_notes: string;
}

export interface PolicyRecord {
  /** Normalized root domain (e.g. example.com) */
  domain: string;
  /** Canonical URL of the policy */
  policy_url: string;
  /** Type of legal document */
  policy_type: PolicyType;
  /** SHA-256 hash of normalized policy text */
  policy_version_hash: string;
  /** Deterministic privacy grade */
  grade: Grade;
  /** Structured extracted summary */
  summary: PolicySummary;
  /** ISO 8601 timestamp when evaluation took place */
  graded_at: string;
  /** SLM or heuristic model version used */
  model_version: string;
  /** Source of the evaluation */
  source: GradeSource;
}

export interface CacheShard {
  /** Cache schema specification version */
  version: string;
  /** Identifier for the shard partition (e.g. shard-001) */
  shard_id: string;
  /** ISO 8601 timestamp when this shard was last updated */
  updated_at: string;
  /** Array of pre-graded policy records in this shard */
  policies: PolicyRecord[];
}

export interface GradeRequest {
  url: string;
  title: string;
  text: string;
}

export interface GradeResponse {
  grade: Grade;
  summary: PolicySummary;
  cached: boolean;
  source: GradeSource;
}
