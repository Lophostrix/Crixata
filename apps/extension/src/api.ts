export type Grade = 'A' | 'B' | 'C' | 'D';

export interface UserRights {
  can_delete_data: boolean;
  can_export_data: boolean;
  can_opt_out_of_tracking: boolean;
}

export interface PolicySummary {
  data_collected: string[];
  data_used_for: string[];
  shared_with_third_parties: boolean;
  third_party_details: string;
  retention_period: string;
  user_rights: UserRights;
  tracking_and_ads: string;
  arbitration_or_class_action_waiver: boolean;
  policy_clarity_notes: string;
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
  source: 'curated' | 'community' | 'llm';
}

export interface HealthResponse {
  status: string;
  ready: boolean;
  sidecar_ready: boolean;
  version?: string;
}

export const DEFAULT_API_URL = 'http://127.0.0.1:4343';

/**
 * Checks whether the desktop companion app and sidecar are running.
 */
export async function checkHealth(apiUrl: string = DEFAULT_API_URL): Promise<HealthResponse> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 2000);

  try {
    const res = await fetch(`${apiUrl}/health`, {
      method: 'GET',
      signal: controller.signal,
    });
    if (!res.ok) {
      throw new Error(`HTTP error ${res.status}`);
    }
    return await res.json();
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Sends policy text to local desktop companion app for inference or cache lookup.
 */
export async function gradePolicy(
  req: GradeRequest,
  apiUrl: string = DEFAULT_API_URL
): Promise<GradeResponse> {
  const controller = new AbortController();
  // Allow up to 45 seconds for local LLM inference
  const timeoutId = setTimeout(() => controller.abort(), 45000);

  try {
    const res = await fetch(`${apiUrl}/grade`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(req),
      signal: controller.signal,
    });

    if (!res.ok) {
      const errBody = await res.text();
      throw new Error(`Server returned ${res.status}: ${errBody}`);
    }

    return await res.json();
  } finally {
    clearTimeout(timeoutId);
  }
}
