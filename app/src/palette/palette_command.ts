export type PaletteAction =
	| { kind: 'newTunnel' }
	| { kind: 'newTemporaryTunnel' }
	| { kind: 'importTunnels' }
	| { kind: 'openSettings' }
	| { kind: 'revealConfig' }
	| { kind: 'toggleSidebar' }
	| { kind: 'showShortcuts' }
	| { kind: 'startTunnel'; tunnelId: string }
	| { kind: 'stopTunnel'; tunnelId: string }
	| { kind: 'showTunnelDetails'; tunnelId: string }
	| { kind: 'editTunnel'; tunnelId: string }
	| { kind: 'openTunnel'; id: string }
	| { kind: 'startTag'; tag: string }
	| { kind: 'stopTag'; tag: string }
	| { kind: 'renameTag'; tag: string }
	| { kind: 'deleteTag'; tag: string }
	| { kind: 'showHostConfig'; host: string }
	| { kind: 'showX11'; host: string };

export type PaletteCategory = 'Général' | 'Tunnel' | 'Tag' | 'Hôte';

export type PaletteCommand = {
	id: string;
	label: string;
	category: PaletteCategory;
	action: PaletteAction;
};
