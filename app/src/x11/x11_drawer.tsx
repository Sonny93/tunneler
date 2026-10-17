import { X11Panel } from '#/x11/x11_panel';
import { useUiStore } from '#/state/ui_store';

/** The X11 drawer, open while the UI panel is `x11`. */
export function X11Drawer() {
	const panel = useUiStore((state) => state.panel);

	if (panel.kind !== 'x11') return null;
	return <X11Panel host={panel.host} />;
}
