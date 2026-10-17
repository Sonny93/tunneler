import { useRef } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useModalCall } from '#/shared/use_modal_call';
import { PaletteContent } from '#/palette/palette_content';
import type { PaletteAction } from '#/palette/palette_command';
import { useRunPaletteAction } from '#/palette/use_run_palette_action';

const PALETTE_MODAL_ID = 'command-palette';

/**
 * Opens the command palette dialog while `isPaletteOpen` is set; a
 * `<ModalProvider />` must be mounted. A chosen command runs once the dialog
 * is closed, so the drawers it opens take the focus the dialog just gave back.
 */
export function CommandPalette(): null {
	const isPaletteOpen = useUiStore((state) => state.isPaletteOpen);
	const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);
	const runAction = useRunPaletteAction();
	const pendingAction = useRef<PaletteAction | null>(null);

	const handleDismissed = () => {
		setPaletteOpen(false);
		const action = pendingAction.current;
		pendingAction.current = null;
		if (action !== null) setTimeout(() => runAction(action), 0);
	};
	const request = isPaletteOpen
		? {
				id: PALETTE_MODAL_ID,
				props: {
					title: 'Palette de commandes',
					size: 'lg' as const,
					closeLabel: 'Fermer',
					children: (close: () => void) => (
						<PaletteContent
							onRun={(action) => {
								pendingAction.current = action;
								close();
							}}
						/>
					),
				},
			}
		: null;
	useModalCall(request, handleDismissed);

	return null;
}
