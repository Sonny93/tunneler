import { X11Drawer } from '#/x11/x11_drawer';
import { PromptHost } from '#/prompts/prompt_host';
import { ImportDrawer } from '#/import/import_drawer';
import { CommandPalette } from '#/palette/command_palette';
import { ShortcutsHelp } from '#/shortcuts/shortcuts_help';
import { SettingsDrawer } from '#/settings/settings_drawer';
import { TagRenameHost } from '#/tag_rename/tag_rename_host';
import { HostConfigDrawer } from '#/host_config/host_config_drawer';
import { TunnelFormDrawer } from '#/tunnel_form/tunnel_form_drawer';
import { useGlobalShortcuts } from '#/shortcuts/use_global_shortcuts';
import { useQuitRequestListener } from '#/quit/use_quit_request_listener';
import { TunnelDetailsDrawer } from '#/tunnel_details/tunnel_details_drawer';
import { useStandbyRequestListener } from '#/standby/use_standby_request_listener';

/** Everything that opens over the main screen; mount it once inside the connected state. */
export function Overlays() {
	useGlobalShortcuts();
	useQuitRequestListener();
	useStandbyRequestListener();

	return (
		<>
			<TunnelFormDrawer />
			<SettingsDrawer />
			<X11Drawer />
			<TunnelDetailsDrawer />
			<HostConfigDrawer />
			<ImportDrawer />
			<TagRenameHost />
			<PromptHost />
			<ShortcutsHelp />
			<CommandPalette />
		</>
	);
}
