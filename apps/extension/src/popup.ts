import { extractDomain } from './detector';
import { GradeResponse } from './api';

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
  if (domainEl) domainEl.textContent = domain || 'Unknown Domain';

  // 2. Check Desktop Companion Health
  checkCompanionStatus();

  // 3. Get existing tab state
  chrome.runtime.sendMessage(
    { type: 'GET_TAB_STATE', tabId: currentTabId },
    (response) => {
      if (response && response.state) {
        renderState(response.state);
      } else {
        renderDefault(domain);
      }
    }
  );

  // 4. Attach Grade Button Listener
  const gradeBtn = document.getElementById('grade-btn');
  if (gradeBtn) {
    gradeBtn.addEventListener('click', handleGradeClick);
  }
}

function checkCompanionStatus(): void {
  const dotEl = document.getElementById('status-dot');
  const textEl = document.getElementById('status-text');

  chrome.runtime.sendMessage({ type: 'CHECK_COMPANION_HEALTH' }, (res) => {
    if (res && res.success && res.health?.ready) {
      if (dotEl) {
        dotEl.className = 'status-dot connected';
      }
      if (textEl) {
        textEl.textContent = 'Desktop Ready';
        textEl.style.color = '#10B981';
      }
    } else {
      if (dotEl) {
        dotEl.className = 'status-dot';
      }
      if (textEl) {
        textEl.textContent = 'Desktop Offline';
        textEl.style.color = '#EF4444';
      }
    }
  });
}

function renderDefault(domain: string): void {
  const typeEl = document.getElementById('policy-type');
  const badgeEl = document.getElementById('grade-badge');
  if (typeEl) typeEl.textContent = 'Web Page';
  if (badgeEl) {
    badgeEl.textContent = '?';
    badgeEl.className = 'grade-badge';
  }
}

function renderState(state: any): void {
  const typeEl = document.getElementById('policy-type');
  const badgeEl = document.getElementById('grade-badge');

  if (typeEl) {
    typeEl.textContent = (state.policyType || 'policy').replace(/_/g, ' ');
  }

  if (state.grade) {
    if (badgeEl) {
      badgeEl.textContent = state.grade;
      badgeEl.className = `grade-badge grade-${state.grade}`;
    }
  }

  if (state.gradeResponse) {
    renderSummary(state.gradeResponse);
  }
}

function renderSummary(res: GradeResponse): void {
  const summaryCard = document.getElementById('summary-card');
  if (!summaryCard) return;

  summaryCard.style.display = 'flex';

  // Data collected tags
  const dataCollectedEl = document.getElementById('data-collected');
  if (dataCollectedEl) {
    dataCollectedEl.innerHTML = '';
    for (const item of res.summary.data_collected || []) {
      const tag = document.createElement('span');
      tag.className = 'tag';
      tag.textContent = item;
      dataCollectedEl.appendChild(tag);
    }
  }

  // Purposes & sharing
  const sharingEl = document.getElementById('third-party-details');
  if (sharingEl) {
    sharingEl.textContent = res.summary.shared_with_third_parties
      ? `⚠️ Third-party sharing: ${res.summary.third_party_details}`
      : '🛡️ No third-party data sharing mentioned.';
  }

  const purposesEl = document.getElementById('data-purposes');
  if (purposesEl) {
    purposesEl.innerHTML = '';
    for (const item of res.summary.data_used_for || []) {
      const tag = document.createElement('span');
      tag.className = 'tag';
      tag.textContent = item;
      purposesEl.appendChild(tag);
    }
  }

  // User rights
  const delEl = document.getElementById('right-delete');
  const expEl = document.getElementById('right-export');
  const optEl = document.getElementById('right-optout');

  if (delEl) {
    delEl.className = `right-item ${res.summary.user_rights?.can_delete_data ? 'yes' : 'no'}`;
  }
  if (expEl) {
    expEl.className = `right-item ${res.summary.user_rights?.can_export_data ? 'yes' : 'no'}`;
  }
  if (optEl) {
    optEl.className = `right-item ${res.summary.user_rights?.can_opt_out_of_tracking ? 'yes' : 'no'}`;
  }

  // Arbitration waiver
  const arbEl = document.getElementById('arbitration-alert');
  if (arbEl) {
    arbEl.style.display = res.summary.arbitration_or_class_action_waiver ? 'block' : 'none';
  }

  // Clarity notes
  const notesEl = document.getElementById('clarity-notes');
  if (notesEl) {
    notesEl.textContent = res.summary.policy_clarity_notes || 'No specific notes available.';
  }
}

async function handleGradeClick(): Promise<void> {
  if (!currentTabId) return;

  const btn = document.getElementById('grade-btn') as HTMLButtonElement;
  const btnText = document.getElementById('btn-text');
  const badgeEl = document.getElementById('grade-badge');

  if (btn) btn.disabled = true;
  if (btnText) btnText.textContent = 'Grading with local SLM...';
  if (badgeEl) {
    badgeEl.textContent = '...';
    badgeEl.className = 'grade-badge';
  }

  chrome.runtime.sendMessage(
    { type: 'GRADE_PAGE', tabId: currentTabId },
    (res) => {
      if (btn) btn.disabled = false;
      if (btnText) btnText.textContent = 'Re-Grade Page';

      if (res && res.success && res.result) {
        const gradeRes = res.result as GradeResponse;
        if (badgeEl) {
          badgeEl.textContent = gradeRes.grade;
          badgeEl.className = `grade-badge grade-${gradeRes.grade}`;
        }
        renderSummary(gradeRes);
      } else {
        showError(res?.error || 'Grading failed. Is Crixata desktop app running?');
        if (badgeEl) {
          badgeEl.textContent = '?';
        }
      }
    }
  );
}

function showError(msg: string): void {
  const typeEl = document.getElementById('policy-type');
  if (typeEl) {
    typeEl.textContent = msg;
    typeEl.style.color = '#EF4444';
  }
}

document.addEventListener('DOMContentLoaded', initPopup);
