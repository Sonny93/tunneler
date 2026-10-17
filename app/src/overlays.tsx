import { X11Drawer } from '#/x11/x11_drawer';
import { PromptHost } from '#/prompts/prompt_host';
import { ShortcutsHelp } from '#/shortcuts/shortcuts_help';
import { SettingsDrawer } from '#/settings/settings_drawer';
import { TunnelFormDrawer } from '#/tunnel_form/tunnel_form_drawer';
import { useGlobalShortcuts } from '#/shortcuts/use_global_shortcuts';
import { useQuitRequestListener } from '#/quit/use_quit_request_listener';

/** Everything that opens over the main screen; mount it once inside the connected state. */
export function Overlays() {
	useGlobalShortcuts();
	useQuitRequestListener();

	return (
		<>
			<TunnelFormDrawer />
			<SettingsDrawer />
			<X11Drawer />
			<PromptHost />
			<ShortcutsHelp />
		</>
	);
}
