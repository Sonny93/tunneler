import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import { useX11State } from '#/x11/use_x11_state';
import { X11SecurityNote } from '#/x11/x11_security_note';
import { X11AnchorSection } from '#/x11/x11_anchor_section';
import { X11DisplaySection } from '#/x11/x11_display_section';
import { X11LaunchersSection } from '#/x11/x11_launchers_section';

type X11PanelProps = {
	host: string;
};

export function X11Panel({ host }: Readonly<X11PanelProps>) {
	const closePanel = useUiStore((state) => state.closePanel);
	const state = useX11State(host);

	return (
		<Drawer isOpen title={`X11 : ${host}`} onClose={closePanel}>
			<div className="flex flex-col gap-6">
				<X11SecurityNote />
				<X11AnchorSection host={host} />
				{state.kind === 'active' && (
					<X11DisplaySection display={state.display} />
				)}
				<X11LaunchersSection host={host} />
			</div>
		</Drawer>
	);
}
