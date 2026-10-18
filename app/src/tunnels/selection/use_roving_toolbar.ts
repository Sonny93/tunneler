import { useState, type KeyboardEvent, type SyntheticEvent } from 'react';

import { resolveToolbarIndex } from '#/tunnels/selection/resolve_toolbar_index';

const TOOLBAR_ITEM_ATTRIBUTE = 'data-toolbar-item';

type ToolbarItemProps = { tabIndex: 0 | -1 } & {
	[TOOLBAR_ITEM_ATTRIBUTE]: string;
};

type UseRovingToolbarReturn = {
	getItemProps: (key: string) => ToolbarItemProps;
	toolbarProps: {
		onFocus: (event: SyntheticEvent<HTMLElement>) => void;
		onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
	};
};

function findItem(target: EventTarget): HTMLElement | null {
	if (!(target instanceof HTMLElement)) return null;
	return target.closest<HTMLElement>(`[${TOOLBAR_ITEM_ATTRIBUTE}]`);
}

/**
 * The keyboard behavior of an ARIA toolbar: a single tab stop (the last focused
 * item, else the first enabled one) and ←, →, Home, End between the enabled
 * items. `enabledKeys` lists the items in DOM order.
 */
export function useRovingToolbar(
	enabledKeys: readonly string[]
): UseRovingToolbarReturn {
	const [requestedKey, setRequestedKey] = useState<string | null>(null);
	const isRequestedKeyEnabled =
		requestedKey !== null && enabledKeys.includes(requestedKey);
	const activeKey = isRequestedKeyEnabled
		? requestedKey
		: (enabledKeys[0] ?? null);

	const handleFocus = (event: SyntheticEvent<HTMLElement>) => {
		const item = findItem(event.target);
		if (item) setRequestedKey(item.getAttribute(TOOLBAR_ITEM_ATTRIBUTE));
	};

	const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
		const item = findItem(event.target);
		if (!item || event.ctrlKey || event.altKey || event.metaKey) return;
		const items = [
			...event.currentTarget.querySelectorAll<HTMLElement>(
				`[${TOOLBAR_ITEM_ATTRIBUTE}]:not(:disabled)`
			),
		];
		const targetIndex = resolveToolbarIndex(
			event.key,
			items.indexOf(item),
			items.length
		);
		if (targetIndex === null) return;
		event.preventDefault();
		items[targetIndex]?.focus();
	};

	return {
		getItemProps: (key) => ({
			tabIndex: key === activeKey ? 0 : -1,
			[TOOLBAR_ITEM_ATTRIBUTE]: key,
		}),
		toolbarProps: { onFocus: handleFocus, onKeyDown: handleKeyDown },
	};
}
