import { useRetainedPanel } from '#/shared/use_retained_panel';
import { useUiStore } from '#/state/ui_store';
import { X11Panel } from '#/x11/x11_panel';

/** The X11 drawer, open while the UI panel is `x11` and kept mounted while it slides out. */
export function X11Drawer() {
	const panel = useUiStore((state) => state.panel);
	const {
		panel: x11Panel,
		isOpen,
		instanceKey,
	} = useRetainedPanel(panel.kind === 'x11' ? panel : null);

	if (x11Panel === null) return null;
	return <X11Panel key={instanceKey} host={x11Panel.host} isOpen={isOpen} />;
}
