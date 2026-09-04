/**
 * Crixata Content Script
 * Extracts policy text from web pages for local grading.
 */

function cleanDocumentText(): string {
  // Find primary article or main container if available
  const container =
    document.querySelector('main') ||
    document.querySelector('article') ||
    document.querySelector('[role="main"]') ||
    document.body;

  if (!container) return '';

  const clone = container.cloneNode(true) as HTMLElement;

  // Remove non-content elements
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

  // Extract clean text
  const rawText = clone.innerText || clone.textContent || '';
  
  // Normalize whitespace
  const normalized = rawText
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+/g, ' ')
    .replace(/\n\s*\n\s*\n+/g, '\n\n')
    .trim();

  // Cap at 64k characters for model context limit
  return normalized.slice(0, 65536);
}

export interface ExtractedPagePayload {
  url: string;
  title: string;
  text: string;
}

function extractPage(): ExtractedPagePayload {
  return {
    url: window.location.href,
    title: document.title || '',
    text: cleanDocumentText(),
  };
}

// Listen for requests from popup or background script
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message && message.type === 'EXTRACT_POLICY_TEXT') {
    const payload = extractPage();
    sendResponse({ success: true, data: payload });
    return true;
  }
});

// Proactively send page info on idle load
try {
  chrome.runtime.sendMessage({
    type: 'CONTENT_LOADED',
    data: {
      url: window.location.href,
      title: document.title || '',
    },
  });
} catch {
  // Extension context might be invalidated or not ready
}
