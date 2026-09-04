import type { DetectionResult, PolicyType, Confidence, CandidateLink } from './types';

export type { DetectionResult, PolicyType, Confidence };

export const POLICY_URL_KEYWORDS = [
  'privacy-policy',
  'privacypolicy',
  'privacy',
  'terms-of-service',
  'terms-of-use',
  'terms',
  'tos',
  'user-agreement',
  'conditions',
  'eula',
  'legal',
  'cookie-policy',
  'cookie',
] as const;

export const POLICY_TITLE_KEYWORDS = [
  'Privacy Policy',
  'Terms of Service',
  'Terms of Use',
  'Terms and Conditions',
  'Legal',
  'Cookie Policy',
] as const;

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
 * Maps a keyword or string to policy type: 'privacy' | 'terms' | 'cookie' | 'unknown'.
 */
export function classifyKeyword(keyword: string): PolicyType {
  const k = keyword.toLowerCase();
  if (k.includes('cookie')) {
    return 'cookie';
  }
  if (k.includes('privacy')) {
    return 'privacy';
  }
  if (
    k.includes('term') ||
    k.includes('tos') ||
    k.includes('eula') ||
    k.includes('condition') ||
    k.includes('agreement') ||
    k.includes('legal')
  ) {
    return 'terms';
  }
  return 'unknown';
}

/**
 * Heuristic policy detector checking URL path, title, and DOM links in order.
 */
export function detectPolicyPage(
  urlStr: string,
  title: string = '',
  links?: CandidateLink[] | Array<{ text: string; href: string }>
): DetectionResult {
  try {
    const url = new URL(urlStr);
    // Ignore non-web protocol pages
    if (!['http:', 'https:'].includes(url.protocol)) {
      return { isPolicyPage: false, policyType: 'unknown', confidence: 'low' };
    }

    const path = url.pathname.toLowerCase();

    // 1. Check URL path
    for (const kw of POLICY_URL_KEYWORDS) {
      if (path.includes(kw)) {
        return {
          isPolicyPage: true,
          policyType: classifyKeyword(kw),
          confidence: 'high',
        };
      }
    }

    // 2. Check Page <title>
    if (title) {
      const titleLower = title.toLowerCase();
      for (const kw of POLICY_TITLE_KEYWORDS) {
        if (titleLower.includes(kw.toLowerCase())) {
          return {
            isPolicyPage: true,
            policyType: classifyKeyword(kw),
            confidence: 'medium',
          };
        }
      }
    }

    // 3. DOM: look for links on the page whose text or href matches the keywords
    if (links && links.length > 0) {
      for (const link of links) {
        const linkText = (link.text || '').toLowerCase();
        const linkHref = (link.href || '').toLowerCase();

        for (const kw of POLICY_URL_KEYWORDS) {
          if (linkText.includes(kw) || linkHref.includes(kw)) {
            return {
              isPolicyPage: true,
              policyType: classifyKeyword(kw),
              confidence: 'low',
            };
          }
        }

        for (const kw of POLICY_TITLE_KEYWORDS) {
          const kwLower = kw.toLowerCase();
          if (linkText.includes(kwLower) || linkHref.includes(kwLower)) {
            return {
              isPolicyPage: true,
              policyType: classifyKeyword(kw),
              confidence: 'low',
            };
          }
        }
      }
    }

    return {
      isPolicyPage: false,
      policyType: 'unknown',
      confidence: 'low',
    };
  } catch {
    return {
      isPolicyPage: false,
      policyType: 'unknown',
      confidence: 'low',
    };
  }
}
