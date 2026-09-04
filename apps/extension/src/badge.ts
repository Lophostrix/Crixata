export type BadgeGrade = 'A' | 'B' | 'C' | 'D' | '?' | '...';

const GRADE_COLORS: Record<BadgeGrade, string> = {
  A: '#10B981', // emerald-500
  B: '#3B82F6', // blue-500
  C: '#F59E0B', // amber-500
  D: '#EF4444', // red-500
  '?': '#6B7280', // gray-500
  '...': '#8B5CF6', // violet-500
};

/**
 * Updates the extension badge text, color, and title for a specific tab.
 */
export async function setBadge(
  tabId: number,
  grade: BadgeGrade | string,
  tooltip?: string
): Promise<void> {
  try {
    const text = grade.toUpperCase();
    const color = GRADE_COLORS[grade as BadgeGrade] || '#6B7280';

    await chrome.action.setBadgeText({ tabId, text });
    await chrome.action.setBadgeBackgroundColor({ tabId, color });

    if (tooltip) {
      await chrome.action.setTitle({ tabId, title: tooltip });
    }
  } catch (error) {
    // Tab might have closed or not exist
    console.debug(`Failed to update badge for tab ${tabId}:`, error);
  }
}

/**
 * Sets badge to loading state (...).
 */
export async function setBadgeLoading(tabId: number): Promise<void> {
  await setBadge(tabId, '...', 'Analyzing policy with Crixata...');
}

/**
 * Clears the badge for a specific tab.
 */
export async function clearBadge(tabId: number): Promise<void> {
  try {
    await chrome.action.setBadgeText({ tabId, text: '' });
    await chrome.action.setTitle({ tabId, title: 'Crixata Policy Grader' });
  } catch (error) {
    console.debug(`Failed to clear badge for tab ${tabId}:`, error);
  }
}
