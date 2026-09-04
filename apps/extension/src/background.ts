import { detectPolicyPage, extractDomain } from './detector';
import { setBadge, clearBadge } from './badge';
import { checkHealth, gradePolicy } from './api';
import type { TabGradeRecord, ExtractedPageText, CandidateLink } from './types';

const THROTTLE_MS = 5 * 60 * 1000; // 5 minutes

function getStorage(): chrome.storage.StorageArea {
  return chrome.storage.session || chrome.storage.local;
}

function isValidWebUrl(urlStr?: string): boolean {
  if (!urlStr) return false;
  return urlStr.startsWith('http://') || urlStr.startsWith('https://');
}

/**
 * Evaluates tab for legal policy content, extracts text, calls companion API, and updates badge/storage.
 */
async function evaluateAndGradeTab(
  tabId: number,
  url: string,
  title: string = '',
  force: boolean = false
): Promise<TabGradeRecord | null> {
  if (!isValidWebUrl(url)) return null;

  const storage = getStorage();
  const throttleKey = `throttle_${url}`;

  // Check 5-minute throttle cache unless user explicitly requested force re-analysis
  if (!force) {
    try {
      const throttleData = await storage.get(throttleKey);
      const cached = throttleData[throttleKey];
      if (cached && Date.now() - cached.timestamp < THROTTLE_MS && cached.record) {
        const record: TabGradeRecord = {
          ...cached.record,
          tabId,
        };
        await storage.set({ [`tab_${tabId}`]: record });
        if (record.isPolicyPage && record.grade) {
          await setBadge(tabId, record.grade);
        } else {
          await clearBadge(tabId);
        }
        return record;
      }
    } catch {
      // Fall through to evaluate
    }
  }

  // 1. Policy Detection Heuristics
  let detection = detectPolicyPage(url, title);
  let candidateLinks: CandidateLink[] = [];

  // If not already high/medium confidence, check DOM links via content script
  try {
    const links = await chrome.tabs.sendMessage(tabId, { action: 'scan-links' });
    if (Array.isArray(links)) candidateLinks = links;
  } catch {
    // If content script was not ready, inject it via chrome.scripting.executeScript
    try {
      await chrome.scripting.executeScript({
        target: { tabId },
        files: ['content.js'],
      });
      const links = await chrome.tabs.sendMessage(tabId, { action: 'scan-links' });
      if (Array.isArray(links)) candidateLinks = links;
    } catch {
      // Tab may not allow scripting
    }
  }

  if (!detection.isPolicyPage && candidateLinks.length > 0) {
    detection = detectPolicyPage(url, title, candidateLinks);
  }

  // If not a policy page
  if (!detection.isPolicyPage) {
    await clearBadge(tabId);
    const nonPolicyRecord: TabGradeRecord = {
      tabId,
      url,
      domain: extractDomain(url),
      title,
      isPolicyPage: false,
      policyType: 'unknown',
      confidence: 'low',
      grade: '?',
      candidateLinks,
      timestamp: Date.now(),
    };
    await storage.set({ [`tab_${tabId}`]: nonPolicyRecord });
    return nonPolicyRecord;
  }

  // 2. Policy page detected: set badge to loading
  await setBadge(tabId, '...');

  // 3. Extract text from page
  let extracted: ExtractedPageText | null = null;
  try {
    extracted = await chrome.tabs.sendMessage(tabId, { action: 'extract-text' });
  } catch {
    try {
      await chrome.scripting.executeScript({
        target: { tabId },
        files: ['content.js'],
      });
      extracted = await chrome.tabs.sendMessage(tabId, { action: 'extract-text' });
    } catch (err) {
      console.debug('Failed to extract text from tab:', err);
    }
  }

  const policyText = extracted?.text || '';
  const pageTitle = extracted?.title || title || '';

  // 4. Check desktop app health
  const health = await checkHealth();
  if (!health.ready) {
    await setBadge(tabId, '?');
    const offlineRecord: TabGradeRecord = {
      tabId,
      url,
      domain: extractDomain(url),
      title: pageTitle,
      isPolicyPage: true,
      policyType: detection.policyType,
      confidence: detection.confidence,
      grade: '?',
      candidateLinks,
      appHealthy: false,
      error: health.error || 'Crixata desktop app is not running.',
      timestamp: Date.now(),
    };
    await storage.set({ [`tab_${tabId}`]: offlineRecord });
    return offlineRecord;
  }

  // 5. Desktop app healthy: grade policy
  try {
    if (!policyText) {
      throw new Error('No readable text content found on this policy page.');
    }

    const gradeResult = await gradePolicy({
      url,
      title: pageTitle,
      text: policyText,
    });

    await setBadge(tabId, gradeResult.grade);

    const gradedRecord: TabGradeRecord = {
      tabId,
      url,
      domain: extractDomain(url),
      title: pageTitle,
      isPolicyPage: true,
      policyType: detection.policyType,
      confidence: detection.confidence,
      grade: gradeResult.grade,
      summary: gradeResult.summary,
      cached: gradeResult.cached,
      source: gradeResult.source,
      candidateLinks,
      appHealthy: true,
      timestamp: Date.now(),
    };

    // Store in tab session and throttle cache
    await storage.set({
      [`tab_${tabId}`]: gradedRecord,
      [throttleKey]: {
        timestamp: Date.now(),
        grade: gradeResult.grade,
        record: gradedRecord,
      },
    });

    return gradedRecord;
  } catch (err: any) {
    await setBadge(tabId, '?');
    const errorRecord: TabGradeRecord = {
      tabId,
      url,
      domain: extractDomain(url),
      title: pageTitle,
      isPolicyPage: true,
      policyType: detection.policyType,
      confidence: detection.confidence,
      grade: '?',
      candidateLinks,
      appHealthy: true,
      error: err?.message || 'Grading failed.',
      timestamp: Date.now(),
    };
    await storage.set({ [`tab_${tabId}`]: errorRecord });
    return errorRecord;
  }
}

// Tab navigation listener: triggers when page load is complete
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' && tab.url) {
    evaluateAndGradeTab(tabId, tab.url, tab.title || '', false);
  }
});

// Restore badge when tab is activated
chrome.tabs.onActivated.addListener(async (activeInfo) => {
  try {
    const tab = await chrome.tabs.get(activeInfo.tabId);
    if (tab.url && isValidWebUrl(tab.url)) {
      const storage = getStorage();
      const data = await storage.get(`tab_${activeInfo.tabId}`);
      const record = data[`tab_${activeInfo.tabId}`] as TabGradeRecord | undefined;
      if (record?.isPolicyPage && record?.grade) {
        await setBadge(activeInfo.tabId, record.grade);
      }
    }
  } catch {}
});

// Clean up stored tab state on tab close
chrome.tabs.onRemoved.addListener((tabId) => {
  try {
    getStorage().remove(`tab_${tabId}`);
  } catch {}
});

// Runtime messages from popup
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message?.action === 'reanalyze' || message?.type === 'GRADE_PAGE') {
    const tabId = message.tabId;
    (async () => {
      try {
        const tab = await chrome.tabs.get(tabId);
        if (!tab.url) throw new Error('Cannot find URL for active tab');
        const result = await evaluateAndGradeTab(tabId, tab.url, tab.title || '', true);
        sendResponse({ success: true, result });
      } catch (err: any) {
        sendResponse({ success: false, error: err?.message || 'Re-analysis failed' });
      }
    })();
    return true;
  }

  if (message?.action === 'get-tab-state' || message?.type === 'GET_TAB_STATE') {
    const tabId = message.tabId;
    getStorage().get(`tab_${tabId}`).then((data) => {
      sendResponse({ state: data[`tab_${tabId}`] || null });
    });
    return true;
  }

  if (message?.action === 'check-health' || message?.type === 'CHECK_COMPANION_HEALTH') {
    checkHealth().then((health) => {
      sendResponse({ success: true, health });
    }).catch((err) => {
      sendResponse({ success: false, error: err?.message });
    });
    return true;
  }
});
