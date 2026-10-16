import { create } from 'zustand';

import type { TunnelSpecDto } from '#/daemon/protocol';

export type Panel =
	| { kind: 'none' }
	| { kind: 'tunnelEditor'; tunnelId: string | null; draft?: TunnelSpecDto }
	| { kind: 'settings' };

export type TunnelFilter =
	| { kind: 'all' }
	| { kind: 'tag'; tag: string }
	| { kind: 'host'; host: string };

export type TrayProblemKind = 'missingHost' | 'failed';

type UiState = {
	panel: Panel;
	filter: TunnelFilter;
	isHelpOpen: boolean;
	/** The tray problem the user hid; a different problem shows the notice again. */
	dismissedTrayProblem: TrayProblemKind | null;
	/** A just-created tunnel whose switch takes the focus once its row is rendered. */
	pendingFocusTunnelId: string | null;
};

type UiActions = {
	/** `null` opens the editor in create mode. */
	openTunnelEditor: (tunnelId: string | null) => void;
	/** Opens the editor in create mode, prefilled from the spec of an external forward. */
	openTunnelImport: (draft: TunnelSpecDto) => void;
	openSettings: () => void;
	closePanel: () => void;
	closePanelFocusingTunnel: (tunnelId: string) => void;
	clearPendingFocus: () => void;
	setFilter: (filter: TunnelFilter) => void;
	setHelpOpen: (isOpen: boolean) => void;
	dismissTrayProblem: (kind: TrayProblemKind) => void;
};

export const INITIAL_UI_STATE: UiState = {
	panel: { kind: 'none' },
	filter: { kind: 'all' },
	isHelpOpen: false,
	dismissedTrayProblem: null,
	pendingFocusTunnelId: null,
};

export const useUiStore = create<UiState & UiActions>()((set) => ({
	...INITIAL_UI_STATE,
	openTunnelEditor: (tunnelId) =>
		set({ panel: { kind: 'tunnelEditor', tunnelId } }),
	openTunnelImport: (draft) =>
		set({ panel: { kind: 'tunnelEditor', tunnelId: null, draft } }),
	openSettings: () => set({ panel: { kind: 'settings' } }),
	closePanel: () => set({ panel: { kind: 'none' } }),
	closePanelFocusingTunnel: (tunnelId) =>
		set({ panel: { kind: 'none' }, pendingFocusTunnelId: tunnelId }),
	clearPendingFocus: () => set({ pendingFocusTunnelId: null }),
	setFilter: (filter) => set({ filter }),
	setHelpOpen: (isHelpOpen) => set({ isHelpOpen }),
	dismissTrayProblem: (dismissedTrayProblem) => set({ dismissedTrayProblem }),
}));
