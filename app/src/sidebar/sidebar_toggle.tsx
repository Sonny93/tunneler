import { useLayoutEffect } from 'react';
import { IconButton } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';
import { formatShortcutTooltip } from '#/shared/shortcut_tooltip';
import { SIDEBAR_ID, SIDEBAR_TOGGLE_ID } from '#/sidebar/sidebar_id';

/** Keeps the keyboard user's place: a collapsing sidebar cannot keep the focus. */
function isFocusInSidebar(): boolean {
	const sidebar = document.getElementById(SIDEBAR_ID);
	return sidebar?.contains(document.activeElement) ?? false;
}

export function SidebarToggle() {
	const { t } = useLingui();
	const isCollapsed = useUiStore((state) => state.isSidebarCollapsed);
	const toggleSidebar = useUiStore((state) => state.toggleSidebar);
	const label = isCollapsed ? t`Show the sidebar` : t`Hide the sidebar`;

	useLayoutEffect(() => {
		if (isCollapsed && isFocusInSidebar())
			document.getElementById(SIDEBAR_TOGGLE_ID)?.focus();
	}, [isCollapsed]);

	return (
		<IconButton
			id={SIDEBAR_TOGGLE_ID}
			icon="i-mdi-dock-left"
			aria-label={label}
			tooltip={{ content: formatShortcutTooltip(label, 'B') }}
			aria-expanded={!isCollapsed}
			aria-controls={SIDEBAR_ID}
			aria-keyshortcuts="b"
			variant="ghost"
			color="neutral"
			onClick={toggleSidebar}
		/>
	);
}
