import { useLayoutEffect } from 'react';

import { useUiStore } from '#/state/ui_store';
import { TooltipIconButton } from '#/shared/tooltip_icon_button';
import { SIDEBAR_ID, SIDEBAR_TOGGLE_ID } from '#/sidebar/sidebar_id';

/** Keeps the keyboard user's place: a collapsing sidebar cannot keep the focus. */
function isFocusInSidebar(): boolean {
	const sidebar = document.getElementById(SIDEBAR_ID);
	return sidebar?.contains(document.activeElement) ?? false;
}

export function SidebarToggle() {
	const isCollapsed = useUiStore((state) => state.isSidebarCollapsed);
	const toggleSidebar = useUiStore((state) => state.toggleSidebar);

	useLayoutEffect(() => {
		if (isCollapsed && isFocusInSidebar())
			document.getElementById(SIDEBAR_TOGGLE_ID)?.focus();
	}, [isCollapsed]);

	return (
		<TooltipIconButton
			id={SIDEBAR_TOGGLE_ID}
			icon="i-mdi-dock-left"
			aria-label={isCollapsed ? 'Afficher le panneau' : 'Masquer le panneau'}
			aria-expanded={!isCollapsed}
			aria-controls={SIDEBAR_ID}
			aria-keyshortcuts="b"
			shortcutHint="B"
			variant="ghost"
			color="neutral"
			onClick={toggleSidebar}
		/>
	);
}
