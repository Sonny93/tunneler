import { useUiStore } from '#/state/ui_store';
import { useModalCall } from '#/shared/use_modal_call';
import { ShortcutsHelpContent } from '#/shortcuts/shortcuts_help_content';

const HELP_MODAL_ID = 'shortcuts-help';

/** Opens the shortcuts dialog while `isHelpOpen` is set; a `<ModalProvider />` must be mounted. */
export function ShortcutsHelp(): null {
	const isHelpOpen = useUiStore((state) => state.isHelpOpen);
	const setHelpOpen = useUiStore((state) => state.setHelpOpen);

	const closeHelp = () => setHelpOpen(false);
	const request = isHelpOpen
		? {
				id: HELP_MODAL_ID,
				props: {
					title: 'Raccourcis clavier',
					size: 'sm' as const,
					closeLabel: 'Fermer',
					children: <ShortcutsHelpContent />,
				},
			}
		: null;
	useModalCall(request, closeHelp);

	return null;
}
