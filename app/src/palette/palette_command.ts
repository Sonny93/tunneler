import type { SettingsTab } from '#/settings/settings_tab';

export type PaletteAction =
	| { kind: 'newTunnel' }
	| { kind: 'newTemporaryTunnel' }
	| { kind: 'importTunnels' }
	| { kind: 'openSettings'; tab?: SettingsTab }
	| { kind: 'revealConfig' }
	| { kind: 'toggleSidebar' }
	| { kind: 'showShortcuts' }
	| { kind: 'openDocumentation' }
	| { kind: 'checkForUpdate' }
	| { kind: 'startTunnel'; tunnelId: string }
	| { kind: 'stopTunnel'; tunnelId: string }
	| { kind: 'standbyTunnel'; tunnelId: string }
	| { kind: 'showTunnelDetails'; tunnelId: string }
	| { kind: 'editTunnel'; tunnelId: string }
	| { kind: 'openTunnel'; id: string; opener?: string }
	| { kind: 'startTag'; tag: string }
	| { kind: 'stopTag'; tag: string }
	| { kind: 'standbyTag'; tag: string }
	| { kind: 'standbyAll' }
	| { kind: 'renameTag'; tag: string }
	| { kind: 'deleteTag'; tag: string }
	| { kind: 'showHostConfig'; host: string }
	| { kind: 'showX11'; host: string };

export type PaletteCommand = {
	id: string;
	label: string;
	/** Already translated: it is shown and searched as is. */
	category: string;
	action: PaletteAction;
};
