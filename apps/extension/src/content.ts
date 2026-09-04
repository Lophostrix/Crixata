import type { ExtractedPageText, CandidateLink } from './types';

const POLICY_KEYWORDS = [
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
  'cookie',
];

/**
 * Extracts clean readable text from document body.
 * Strips script, style, nav, footer, header, aside, and non-content elements.
 * Normalizes whitespace and limits text to ~15,000 characters.
 */
function cleanDocumentText(): string {
  const container =
    document.querySelector('main') ||
    document.querySelector('article') ||
    document.querySelector('[role="main"]') ||
    document.body;

  if (!container) return '';

  const clone = container.cloneNode(true) as HTMLElement;

  const removeSelectors = [
    'script',
    'style',
    'nav',
    'footer',
    'header',
    'aside',
    'noscript',
    'svg',
    'iframe',
    'form',
    'button',
    '[aria-hidden="true"]',
  ];

  for (const selector of removeSelectors) {
    const elements = clone.querySelectorAll(selector);
    elements.forEach((el) => el.remove());
  }

  const rawText = clone.innerText || clone.textContent || '';

  const normalized = rawText
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+/g, ' ')
    .replace(/\n\s*\n\s*\n+/g, '\n\n')
    .trim();

  // Limit to ~15,000 characters
  return normalized.slice(0, 15000);
}

/**
 * Scans document for links whose text or href matches policy keywords.
 */
function scanCandidateLinks(): CandidateLink[] {
  const anchors = document.querySelectorAll<HTMLAnchorElement>('a[href]');
  const found: CandidateLink[] = [];
  const seenUrls = new Set<string>();

  for (const a of anchors) {
    const href = a.href;
    const text = (a.innerText || a.textContent || '').trim();

    if (!href || href.startsWith('javascript:') || href.startsWith('#')) {
      continue;
    }

    const hrefLower = href.toLowerCase();
    const textLower = text.toLowerCase();

    const matches = POLICY_KEYWORDS.some(
      (kw) => hrefLower.includes(kw) || textLower.includes(kw)
    );

    if (matches && !seenUrls.has(href)) {
      seenUrls.add(href);
      found.push({
        text: text || href,
        href,
      });
      if (found.length >= 30) break;
    }
  }

  return found;
}

// Listen for messages from service worker
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.action === 'extract-text' || message?.type === 'EXTRACT_POLICY_TEXT') {
    const payload: ExtractedPageText = {
      text: cleanDocumentText(),
      title: document.title || '',
      url: window.location.href,
    };
    sendResponse(payload);
    return true;
  }

  if (message?.action === 'scan-links' || message?.type === 'SCAN_LINKS') {
    const links = scanCandidateLinks();
    sendResponse(links);
    return true;
  }
});
