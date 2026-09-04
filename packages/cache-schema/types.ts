/**
 * Crixata Cache Schema & Types
 * Mirror of packages/cache-schema/schema.json & shard.schema.json
 */

export type Grade = 'A' | 'B' | 'C' | 'D';

export type PolicyType =
  | 'privacy_policy'
  | 'terms_of_service'
  | 'cookie_policy'
  | 'other';

export type GradeSource = 'cache' | 'curated' | 'community' | 'llm';

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

export interface CacheShardEntry {
  /** Normalized root domain (e.g. example.com) */
  domain: string;
  /** Canonical URL of the policy */
  policy_url: string;
  /** SHA-256 or text hash of normalized policy text */
  policy_hash: string;
  /** Deterministic privacy grade */
  grade: Grade;
  /** Structured extracted summary */
  summary: PolicySummary;
  /** Source of the evaluation (e.g. cache, curated, community, llm) */
  source: GradeSource | string;
  /** ISO 8601 timestamp when evaluation took place */
  graded_at: string;
  /** Optional type of legal document */
  policy_type?: PolicyType;
  /** Optional SLM or heuristic model version used */
  model_version?: string;
}

/** Alias for backwards compatibility */
export type PolicyRecord = CacheShardEntry;

export interface CacheShard {
  /** Cache schema specification version (e.g. 1) */
  version: number;
  /** ISO 8601 timestamp when this shard was last updated */
  updated_at: string;
  /** Array of pre-graded policy entries in this shard */
  shards: CacheShardEntry[];
}

export interface CacheIndex {
  /** List of available shard filenames (e.g. ["shard-000.json"]) */
  shards: string[];
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
