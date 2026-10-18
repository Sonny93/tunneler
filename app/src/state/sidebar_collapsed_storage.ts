const STORAGE_KEY = 'tunneler.sidebarCollapsed';
const COLLAPSED_VALUE = 'true';

/** Whether the viewer collapsed the sidebar last time; expanded when storage is unavailable. */
export function readSidebarCollapsed(): boolean {
	try {
		return localStorage.getItem(STORAGE_KEY) === COLLAPSED_VALUE;
	} catch {
		return false;
	}
}

/** Best effort: a blocked storage only loses the convenience of remembering. */
export function writeSidebarCollapsed(isCollapsed: boolean): void {
	try {
		if (isCollapsed) localStorage.setItem(STORAGE_KEY, COLLAPSED_VALUE);
		else localStorage.removeItem(STORAGE_KEY);
	} catch {
		return;
	}
}
