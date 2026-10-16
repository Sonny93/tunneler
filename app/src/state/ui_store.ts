import { create } from 'zustand';

export type Panel =
	| { kind: 'none' }
	| { kind: 'tunnelEditor'; tunnelId: string | null }
	| { kind: 'settings' };

export type TunnelFilter =
	| { kind: 'all' }
	| { kind: 'tag'; tag: string }
	| { kind: 'host'; host: string };

type UiState = {
	panel: Panel;
	filter: TunnelFilter;
	isHelpOpen: boolean;
	/** A just-created tunnel whose switch takes the focus once its row is rendered. */
	pendingFocusTunnelId: string | null;
};

type UiActions = {
	/** `null` opens the editor in create mode. */
	openTunnelEditor: (tunnelId: string | null) => void;
	openSettings: () => void;
	closePanel: () => void;
	closePanelFocusingTunnel: (tunnelId: string) => void;
	clearPendingFocus: () => void;
	setFilter: (filter: TunnelFilter) => void;
	setHelpOpen: (isOpen: boolean) => void;
};

export const INITIAL_UI_STATE: UiState = {
	panel: { kind: 'none' },
	filter: { kind: 'all' },
	isHelpOpen: false,
	pendingFocusTunnelId: null,
};

export const useUiStore = create<UiState & UiActions>()((set) => ({
	...INITIAL_UI_STATE,
	openTunnelEditor: (tunnelId) =>
		set({ panel: { kind: 'tunnelEditor', tunnelId } }),
	openSettings: () => set({ panel: { kind: 'settings' } }),
	closePanel: () => set({ panel: { kind: 'none' } }),
	closePanelFocusingTunnel: (tunnelId) =>
		set({ panel: { kind: 'none' }, pendingFocusTunnelId: tunnelId }),
	clearPendingFocus: () => set({ pendingFocusTunnelId: null }),
	setFilter: (filter) => set({ filter }),
	setHelpOpen: (isHelpOpen) => set({ isHelpOpen }),
}));
