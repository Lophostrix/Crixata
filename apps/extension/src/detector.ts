export type PolicyType = 'privacy_policy' | 'terms_of_service' | 'cookie_policy' | 'other';

export interface DetectionResult {
  isPolicy: boolean;
  policyType?: PolicyType;
  confidence: number;
  reason: string;
}

const PRIVACY_URL_PATTERNS = [
  /\/privacy(?:[-_]?(?:policy|notice|statement|info))?/i,
  /\/data[-_]?protection/i,
  /\/legal\/privacy/i,
];

const TERMS_URL_PATTERNS = [
  /\/terms(?:[-_]?(?:of[-_]?(?:service|use)|and[-_]?conditions))?/i,
  /\/tos(?:[-_]?(?:and[-_]?conditions))?/i,
  /\/user[-_]?agreement/i,
  /\/conditions[-_]?of[-_]?(?:use|sale)/i,
  /\/legal\/terms/i,
];

const COOKIE_URL_PATTERNS = [
  /\/cookie(?:[-_]?(?:policy|notice|statement))?/i,
  /\/legal\/cookies/i,
];

const PRIVACY_TITLE_PATTERNS = [
  /privacy\s+policy/i,
  /privacy\s+notice/i,
  /privacy\s+statement/i,
  /data\s+protection\s+notice/i,
];

const TERMS_TITLE_PATTERNS = [
  /terms\s+of\s+service/i,
  /terms\s+of\s+use/i,
  /terms\s+and\s+conditions/i,
  /user\s+agreement/i,
  /conditions\s+of\s+use/i,
];

const COOKIE_TITLE_PATTERNS = [
  /cookie\s+policy/i,
  /cookie\s+notice/i,
];

/**
 * Normalizes and extracts root domain from a URL.
 */
export function extractDomain(urlStr: string): string {
  try {
    const url = new URL(urlStr);
    return url.hostname.replace(/^www\./i, '');
  } catch {
    return '';
  }
}

/**
 * Evaluates whether a URL and optional page title represent a legal policy page.
 */
export function detectPolicyPage(urlStr: string, title: string = ''): DetectionResult {
  try {
    const url = new URL(urlStr);
    // Ignore internal chrome/about/file pages
    if (!['http:', 'https:'].includes(url.protocol)) {
      return { isPolicy: false, confidence: 0, reason: 'Non-web protocol' };
    }

    const path = url.pathname.toLowerCase();
    const search = url.search.toLowerCase();
    const fullTarget = `${path}${search}`;

    // 1. Check URL patterns
    for (const pattern of PRIVACY_URL_PATTERNS) {
      if (pattern.test(fullTarget)) {
        return { isPolicy: true, policyType: 'privacy_policy', confidence: 0.95, reason: 'URL matches privacy policy path' };
      }
    }

    for (const pattern of TERMS_URL_PATTERNS) {
      if (pattern.test(fullTarget)) {
        return { isPolicy: true, policyType: 'terms_of_service', confidence: 0.95, reason: 'URL matches terms path' };
      }
    }

    for (const pattern of COOKIE_URL_PATTERNS) {
      if (pattern.test(fullTarget)) {
        return { isPolicy: true, policyType: 'cookie_policy', confidence: 0.90, reason: 'URL matches cookie policy path' };
      }
    }

    // 2. Check title patterns if URL was not definitive
    if (title) {
      for (const pattern of PRIVACY_TITLE_PATTERNS) {
        if (pattern.test(title)) {
          return { isPolicy: true, policyType: 'privacy_policy', confidence: 0.85, reason: 'Title matches privacy policy' };
        }
      }

      for (const pattern of TERMS_TITLE_PATTERNS) {
        if (pattern.test(title)) {
          return { isPolicy: true, policyType: 'terms_of_service', confidence: 0.85, reason: 'Title matches terms of service' };
        }
      }

      for (const pattern of COOKIE_TITLE_PATTERNS) {
        if (pattern.test(title)) {
          return { isPolicy: true, policyType: 'cookie_policy', confidence: 0.80, reason: 'Title matches cookie policy' };
        }
      }
    }

    return { isPolicy: false, confidence: 0, reason: 'No policy patterns matched' };
  } catch {
    return { isPolicy: false, confidence: 0, reason: 'Invalid URL' };
  }
}
