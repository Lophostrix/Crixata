import { detectPolicyPage, extractDomain } from './detector';
import { setBadge, clearBadge } from './badge';
import { checkHealth, gradePolicy, GradeResponse, HealthResponse } from './api';

interface TabState {
  url: string;
  domain: string;
  title: string;
  isPolicy: boolean;
  policyType?: string;
  grade?: string;
  gradeResponse?: GradeResponse;
  status: 'idle' | 'detecting' | 'grading' | 'done' | 'error';
  error?: string;
}

// In-memory tab states keyed by tabId
const tabStates = new Map<number, TabState>();

async function evaluateTab(tabId: number, url?: string, title?: string): Promise<void> {
  if (!url) return;

  const domain = extractDomain(url);
  const detection = detectPolicyPage(url, title || '');

  if (detection.isPolicy) {
    const currentState = tabStates.get(tabId);
    // Preserve existing grade if url hasn't changed
    if (currentState && currentState.url === url && currentState.grade) {
      await setBadge(tabId, currentState.grade, `Crixata Grade: ${currentState.grade}`);
      return;
    }

    tabStates.set(tabId, {
      url,
      domain,
      title: title || '',
      isPolicy: true,
      policyType: detection.policyType,
      status: 'idle',
      grade: '?',
    });

    await setBadge(tabId, '?', `Detected ${detection.policyType || 'Policy'}`);

    // Try checking desktop app / cache in background
    try {
      const health = await checkHealth();
      if (health.ready) {
        // We can request active tab extraction
        try {
          const res = await chrome.tabs.sendMessage(tabId, { type: 'EXTRACT_POLICY_TEXT' });
          if (res?.data?.text) {
            await setBadge(tabId, '...', 'Analyzing policy...');
            const gradeResult = await gradePolicy({
              url: res.data.url,
              title: res.data.title,
              text: res.data.text,
            });

            tabStates.set(tabId, {
              url,
              domain,
              title: title || '',
              isPolicy: true,
              policyType: detection.policyType,
              grade: gradeResult.grade,
              gradeResponse: gradeResult,
              status: 'done',
            });

            await setBadge(tabId, gradeResult.grade, `Crixata Grade: ${gradeResult.grade}`);
          }
        } catch {
          // Content script might not be injected yet
        }
      }
    } catch {
      // Desktop app might not be running yet, leave '?'
    }
  } else {
    tabStates.delete(tabId);
    await clearBadge(tabId);
  }
}

// Tab navigation listener
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' || changeInfo.url) {
    evaluateTab(tabId, tab.url, tab.title);
  }
});

// Tab switch listener
chrome.tabs.onActivated.addListener(async (activeInfo) => {
  try {
    const tab = await chrome.tabs.get(activeInfo.tabId);
    if (tab.url) {
      evaluateTab(activeInfo.tabId, tab.url, tab.title);
    }
  } catch {
    // Ignore errors for closed tabs
  }
});

// Clean up state on tab removal
chrome.tabs.onRemoved.addListener((tabId) => {
  tabStates.delete(tabId);
});

// Message listener for popup & content script
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'CONTENT_LOADED' && sender.tab?.id) {
    evaluateTab(sender.tab.id, message.data.url, message.data.title);
    sendResponse({ received: true });
    return true;
  }

  if (message.type === 'GET_TAB_STATE') {
    const tabId = message.tabId;
    const state = tabStates.get(tabId);
    sendResponse({ state: state || null });
    return true;
  }

  if (message.type === 'CHECK_COMPANION_HEALTH') {
    checkHealth()
      .then((health: HealthResponse) => sendResponse({ success: true, health }))
      .catch((err: Error) => sendResponse({ success: false, error: err.message }));
    return true;
  }

  if (message.type === 'GRADE_PAGE') {
    const tabId = message.tabId;
    (async () => {
      try {
        await setBadge(tabId, '...', 'Analyzing policy...');
        // 1. Request text from content script
        const extractRes = await chrome.tabs.sendMessage(tabId, { type: 'EXTRACT_POLICY_TEXT' });
        if (!extractRes || !extractRes.data || !extractRes.data.text) {
          throw new Error('Could not extract text from current page');
        }

        // 2. Call local HTTP API
        const gradeResult = await gradePolicy({
          url: extractRes.data.url,
          title: extractRes.data.title,
          text: extractRes.data.text,
        });

        const currentState = tabStates.get(tabId) || {
          url: extractRes.data.url,
          domain: extractDomain(extractRes.data.url),
          title: extractRes.data.title,
          isPolicy: true,
          status: 'idle',
        };

        const updated: TabState = {
          ...currentState,
          grade: gradeResult.grade,
          gradeResponse: gradeResult,
          status: 'done',
        };

        tabStates.set(tabId, updated);
        await setBadge(tabId, gradeResult.grade, `Grade: ${gradeResult.grade}`);
        sendResponse({ success: true, result: gradeResult });
      } catch (err: any) {
        await setBadge(tabId, '?', 'Grading failed');
        sendResponse({ success: false, error: err?.message || 'Grading failed' });
      }
    })();
    return true;
  }
});
