import type { GradeRequest, GradeResponse, Grade, PolicySummary, GradeSource } from './types';

export type { GradeRequest, GradeResponse, Grade, PolicySummary, GradeSource };

export interface HealthResponse {
  status: string;
  ready: boolean;
  sidecar_ready?: boolean;
  model_downloaded?: boolean;
  error?: string;
}

export const DEFAULT_API_URL = 'http://127.0.0.1:4343';

/**
 * Checks whether the desktop companion app and sidecar are running.
 * Handles network errors gracefully when the desktop app is offline.
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
      return {
        status: 'error',
        ready: false,
        error: `Desktop app returned HTTP ${res.status}`,
      };
    }
    const data = await res.json();
    return {
      status: data.status || 'ok',
      ready: data.status === 'ok' && data.sidecar_ready !== false,
      sidecar_ready: data.sidecar_ready,
      model_downloaded: data.model_downloaded,
    };
  } catch (err: any) {
    return {
      status: 'offline',
      ready: false,
      error: 'Desktop companion app is not running or unreachable at 127.0.0.1:4343',
    };
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Sends policy text to local desktop companion app for inference or cache lookup.
 * Handles network errors gracefully with clear descriptive messages.
 */
export async function gradePolicy(
  req: GradeRequest,
  apiUrl: string = DEFAULT_API_URL
): Promise<GradeResponse> {
  const controller = new AbortController();
  // Allow up to 60 seconds for local LLM inference
  const timeoutId = setTimeout(() => controller.abort(), 60000);

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
      let errDetail = '';
      try {
        const json = await res.json();
        errDetail = json.error || JSON.stringify(json);
      } catch {
        errDetail = await res.text();
      }
      throw new Error(`Grading service returned HTTP ${res.status}: ${errDetail}`);
    }

    return await res.json();
  } catch (err: any) {
    if (err.name === 'AbortError') {
      throw new Error('Grading request timed out. Local model took longer than 60s.');
    }
    if (err.message && err.message.includes('Grading service returned')) {
      throw err;
    }
    throw new Error(`Cannot reach Crixata desktop app at ${apiUrl}. Please make sure Crixata is running.`);
  } finally {
    clearTimeout(timeoutId);
  }
}
