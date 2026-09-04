import { extractDomain } from './detector';
import { checkHealth } from './api';
import type { TabGradeRecord, PolicySummary, CandidateLink } from './types';

const storage = chrome.storage.session || chrome.storage.local;

let currentTabId: number | null = null;
let currentTabUrl: string = '';

async function initPopup(): Promise<void> {
  // 1. Query active tab
  const [activeTab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!activeTab || !activeTab.id || !activeTab.url) {
    showError('No active webpage detected.');
    return;
  }

  currentTabId = activeTab.id;
  currentTabUrl = activeTab.url;

  const domain = extractDomain(currentTabUrl);
  const domainEl = document.getElementById('domain-name');
  const urlEl = document.getElementById('policy-url');

  if (domainEl) domainEl.textContent = domain || 'Unknown Domain';
  if (urlEl) urlEl.textContent = currentTabUrl;

  // 2. Check Desktop Companion Health
  checkDesktopStatus();

  // 3. Read cached tab state from storage
  const storageKey = `tab_${currentTabId}`;
  try {
    const data = await storage.get(storageKey);
    const existingRecord = data[storageKey] as TabGradeRecord | undefined;
    if (existingRecord) {
      renderRecord(existingRecord);
    } else {
      // Ask background script if storage hasn't caught up
      chrome.runtime.sendMessage(
        { action: 'get-tab-state', tabId: currentTabId },
        (res) => {
          if (res && res.state) {
            renderRecord(res.state);
          } else {
            renderDefault();
          }
        }
      );
    }
  } catch {
    renderDefault();
  }

  // 4. Listen for storage changes in real-time
  chrome.storage.onChanged.addListener((changes) => {
    if (changes[storageKey]?.newValue) {
      renderRecord(changes[storageKey].newValue as TabGradeRecord);
    }
  });

  // 5. Attach Re-analyze Button Listener
  const reanalyzeBtn = document.getElementById('reanalyze-btn');
  if (reanalyzeBtn) {
    reanalyzeBtn.addEventListener('click', handleReanalyzeClick);
  }
}

async function checkDesktopStatus(): Promise<void> {
  const dotEl = document.getElementById('status-dot');
  const textEl = document.getElementById('status-text');
  const offlineAlert = document.getElementById('offline-alert');

  try {
    const health = await checkHealth();
    if (health.ready) {
      if (dotEl) dotEl.className = 'status-dot connected';
      if (textEl) {
        textEl.textContent = 'Desktop Ready';
        textEl.style.color = '#22c55e';
      }
      if (offlineAlert) offlineAlert.style.display = 'none';
    } else {
      if (dotEl) dotEl.className = 'status-dot';
      if (textEl) {
        textEl.textContent = 'Desktop Offline';
        textEl.style.color = '#ef4444';
      }
      if (offlineAlert) offlineAlert.style.display = 'block';
    }
  } catch {
    if (dotEl) dotEl.className = 'status-dot';
    if (textEl) {
      textEl.textContent = 'Desktop Offline';
      textEl.style.color = '#ef4444';
    }
    if (offlineAlert) offlineAlert.style.display = 'block';
  }
}

function renderDefault(): void {
  const typeEl = document.getElementById('policy-type');
  const badgeEl = document.getElementById('grade-badge');
  const nonPolicy = document.getElementById('non-policy-notice');
  const summaryCard = document.getElementById('summary-card');

  if (typeEl) typeEl.textContent = 'Web Page';
  if (badgeEl) {
    badgeEl.textContent = '?';
    badgeEl.className = 'grade-badge';
  }
  if (nonPolicy) nonPolicy.style.display = 'none';
  if (summaryCard) summaryCard.style.display = 'none';
}

function renderRecord(record: TabGradeRecord): void {
  const typeEl = document.getElementById('policy-type');
  const badgeEl = document.getElementById('grade-badge');
  const sourceBadge = document.getElementById('source-badge');
  const nonPolicyNotice = document.getElementById('non-policy-notice');
  const summaryCard = document.getElementById('summary-card');
  const offlineAlert = document.getElementById('offline-alert');

  // Policy type
  if (typeEl) {
    if (record.isPolicyPage) {
      typeEl.textContent = `${record.policyType || 'policy'} document`.replace(/_/g, ' ');
    } else {
      typeEl.textContent = 'Standard Page';
    }
  }

  // Grade badge
  if (badgeEl) {
    const grade = record.grade || '?';
    badgeEl.textContent = grade;
    if (grade === '...') {
      badgeEl.className = 'grade-badge grade-loading';
    } else {
      badgeEl.className = `grade-badge grade-${grade}`;
    }
  }

  // Source label
  if (sourceBadge) {
    if (record.grade && record.grade !== '?' && record.grade !== '...') {
      sourceBadge.style.display = 'inline-block';
      if (record.source === 'llm') {
        sourceBadge.textContent = 'Analyzed locally';
        sourceBadge.className = 'source-badge source-llm';
      } else {
        sourceBadge.textContent = 'From cache';
        sourceBadge.className = 'source-badge source-cache';
      }
    } else {
      sourceBadge.style.display = 'none';
    }
  }

  // Candidate links
  renderCandidateLinks(record.candidateLinks || []);

  // Summary breakdown
  if (record.isPolicyPage && record.summary) {
    if (nonPolicyNotice) nonPolicyNotice.style.display = 'none';
    if (summaryCard) summaryCard.style.display = 'flex';
    renderSummary(record.summary);
  } else if (!record.isPolicyPage) {
    if (summaryCard) summaryCard.style.display = 'none';
    if (nonPolicyNotice) nonPolicyNotice.style.display = 'block';
  } else {
    // Policy detected but grading pending or offline
    if (summaryCard) summaryCard.style.display = 'none';
    if (nonPolicyNotice) nonPolicyNotice.style.display = 'none';
    if (record.appHealthy === false && offlineAlert) {
      offlineAlert.style.display = 'block';
    }
  }
}

function renderSummary(summary: PolicySummary): void {
  // 1. Data collected
  const dataCollectedEl = document.getElementById('data-collected');
  if (dataCollectedEl) {
    dataCollectedEl.innerHTML = '';
    const items = summary.data_collected || [];
    if (items.length > 0) {
      for (const item of items) {
        const tag = document.createElement('span');
        tag.className = 'tag';
        tag.textContent = item;
        dataCollectedEl.appendChild(tag);
      }
    } else {
      dataCollectedEl.innerHTML = '<span class="info-text">None explicitly specified.</span>';
    }
  }

  // 2. Data used for
  const purposesEl = document.getElementById('data-purposes');
  if (purposesEl) {
    purposesEl.innerHTML = '';
    const items = summary.data_used_for || [];
    if (items.length > 0) {
      for (const item of items) {
        const tag = document.createElement('span');
        tag.className = 'tag';
        tag.textContent = item;
        purposesEl.appendChild(tag);
      }
    } else {
      purposesEl.innerHTML = '<span class="info-text">None explicitly specified.</span>';
    }
  }

  // 3. Third party sharing
  const sharingEl = document.getElementById('third-party-details');
  if (sharingEl) {
    if (summary.shared_with_third_parties) {
      sharingEl.textContent = `⚠️ Shared with third parties: ${summary.third_party_details || 'Yes'}`;
      sharingEl.style.color = '#fca5a5';
    } else {
      sharingEl.textContent = '🛡️ No personal data shared with or sold to third parties.';
      sharingEl.style.color = '#86efac';
    }
  }

  // 4. Data retention
  const retentionEl = document.getElementById('retention-period');
  if (retentionEl) {
    retentionEl.textContent = summary.retention_period || 'Retention duration not explicitly specified in policy.';
  }

  // 5. User rights
  const delEl = document.getElementById('right-delete');
  const expEl = document.getElementById('right-export');
  const optEl = document.getElementById('right-optout');

  if (delEl) {
    const ok = !!summary.user_rights?.can_delete_data;
    delEl.className = `right-item ${ok ? 'yes' : 'no'}`;
    delEl.title = ok ? 'User can request data deletion' : 'No explicit data deletion right';
  }
  if (expEl) {
    const ok = !!summary.user_rights?.can_export_data;
    expEl.className = `right-item ${ok ? 'yes' : 'no'}`;
    expEl.title = ok ? 'User can download / export data' : 'No explicit data export right';
  }
  if (optEl) {
    const ok = !!summary.user_rights?.can_opt_out_of_tracking;
    optEl.className = `right-item ${ok ? 'yes' : 'no'}`;
    optEl.title = ok ? 'User can opt out of tracking / ads' : 'No tracking opt-out option';
  }

  // 6. Tracking & ads
  const trackingEl = document.getElementById('tracking-ads');
  if (trackingEl) {
    trackingEl.textContent = summary.tracking_and_ads || 'No tracking or cookie disclosures specified.';
  }

  // 7. Arbitration waiver
  const arbEl = document.getElementById('arbitration-alert');
  if (arbEl) {
    arbEl.style.display = summary.arbitration_or_class_action_waiver ? 'block' : 'none';
  }

  // 8. Clarity notes
  const notesEl = document.getElementById('clarity-notes');
  if (notesEl) {
    notesEl.textContent = summary.policy_clarity_notes || 'No specific clarity concerns noted.';
  }
}

function renderCandidateLinks(links: CandidateLink[]): void {
  const card = document.getElementById('candidate-links-card');
  const list = document.getElementById('candidate-links-list');
  if (!card || !list) return;

  if (links.length === 0) {
    card.style.display = 'none';
    return;
  }

  card.style.display = 'block';
  list.innerHTML = '';

  for (const link of links) {
    const item = document.createElement('a');
    item.className = 'candidate-link-item';
    item.href = link.href;
    item.target = '_blank';
    item.rel = 'noreferrer noopener';
    item.innerHTML = `<span>${escapeHtml(link.text)}</span><span>↗</span>`;
    list.appendChild(item);
  }
}

async function handleReanalyzeClick(): Promise<void> {
  if (!currentTabId) return;

  const btn = document.getElementById('reanalyze-btn') as HTMLButtonElement | null;
  const btnText = document.getElementById('btn-text');
  const badgeEl = document.getElementById('grade-badge');

  if (btn) btn.disabled = true;
  if (btnText) btnText.textContent = 'Analyzing policy with local SLM...';
  if (badgeEl) {
    badgeEl.textContent = '...';
    badgeEl.className = 'grade-badge grade-loading';
  }

  chrome.runtime.sendMessage(
    { action: 'reanalyze', tabId: currentTabId },
    (res) => {
      if (btn) btn.disabled = false;
      if (btnText) btnText.textContent = 'Re-analyze this page';

      if (res && res.success && res.result) {
        renderRecord(res.result as TabGradeRecord);
      } else {
        showError(res?.error || 'Analysis failed. Is the Crixata desktop app running?');
        checkDesktopStatus();
      }
    }
  );
}

function showError(msg: string): void {
  const typeEl = document.getElementById('policy-type');
  if (typeEl) {
    typeEl.textContent = msg;
    typeEl.style.color = '#ef4444';
  }
}

function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

document.addEventListener('DOMContentLoaded', initPopup);
