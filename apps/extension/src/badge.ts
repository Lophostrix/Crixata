export type BadgeGrade = 'A' | 'B' | 'C' | 'D' | '?' | '...' | string;

export const GRADE_COLORS: Record<string, string> = {
  A: '#22c55e',
  B: '#3b82f6',
  C: '#f97316',
  D: '#ef4444',
  '?': '#6b7280',
  '...': '#8b5cf6',
};

/**
 * Updates the extension badge text, color, and title for a specific tab.
 */
export async function setBadge(
  tabId: number,
  grade: BadgeGrade,
  tooltip?: string
): Promise<void> {
  try {
    const text = (grade || '').toUpperCase();
    const color = GRADE_COLORS[text] || '#6b7280';

    await chrome.action.setBadgeText({ tabId, text });
    await chrome.action.setBadgeBackgroundColor({ tabId, color });

    if (tooltip) {
      await chrome.action.setTitle({ tabId, title: tooltip });
    } else {
      let title = 'Crixata Policy Grader';
      if (text === '?') {
        title = 'Crixata: Desktop app offline or unknown grade';
      } else if (text === '...') {
        title = 'Crixata: Analyzing policy with local SLM...';
      } else if (text) {
        title = `Crixata Grade: ${text}`;
      }
      await chrome.action.setTitle({ tabId, title });
    }
  } catch (error) {
    // Tab might have closed
    console.debug(`Failed to update badge for tab ${tabId}:`, error);
  }
}

/**
 * Resets the badge to empty or '?'.
 */
export async function clearBadge(tabId: number, resetToQuestion: boolean = false): Promise<void> {
  try {
    if (resetToQuestion) {
      await setBadge(tabId, '?');
    } else {
      await chrome.action.setBadgeText({ tabId, text: '' });
      await chrome.action.setTitle({ tabId, title: 'Crixata Policy Grader' });
    }
  } catch (error) {
    console.debug(`Failed to clear badge for tab ${tabId}:`, error);
  }
}
