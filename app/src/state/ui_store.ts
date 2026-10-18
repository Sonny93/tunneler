import { create } from 'zustand';

import type { TunnelSpecDto } from '#/daemon/protocol';
import { getRangeIds } from '#/tunnels/selection/get_range_ids';

export type Panel =
	| { kind: 'none' }
	| { kind: 'tunnelEditor'; tunnelId: string | null; draft?: TunnelSpecDto }
	| { kind: 'settings' }
	| { kind: 'x11'; host: string }
	| { kind: 'tunnelDetails'; tunnelId: string }
	| { kind: 'hostConfig'; host: string }
	| { kind: 'import' };

export type TunnelFilter =
	| { kind: 'all' }
	| { kind: 'tag'; tag: string }
	| { kind: 'host'; host: string };

export type TrayProblemKind = 'missingHost' | 'failed';

type UiState = {
	panel: Panel;
	filter: TunnelFilter;
	isHelpOpen: boolean;
	isPaletteOpen: boolean;
	/** The tray problem the user hid; a different problem shows the notice again. */
	dismissedTrayProblem: TrayProblemKind | null;
	/** A just-created tunnel whose switch takes the focus once its row is rendered. */
	pendingFocusTunnelId: string | null;
	/** Hosts whose X11 risk the user already confirmed during this UI session. */
	confirmedX11Hosts: readonly string[];
	/** Tunnels ticked for bulk actions; read through `useTunnelSelection`, which keeps only the visible ones. */
	selectedTunnelIds: ReadonlySet<string>;
	/** The tunnel ticked last: the start of a Shift+click range. */
	selectionAnchorId: string | null;
};

type UiActions = {
	/** `null` opens the editor in create mode. */
	openTunnelEditor: (tunnelId: string | null) => void;
	/** Opens the editor in create mode, prefilled from the spec of an external forward. */
	openTunnelImport: (draft: TunnelSpecDto) => void;
	openSettings: () => void;
	openX11: (host: string) => void;
	openTunnelDetails: (tunnelId: string) => void;
	openHostConfig: (host: string) => void;
	openImport: () => void;
	confirmX11Host: (host: string) => void;
	closePanel: () => void;
	closePanelFocusingTunnel: (tunnelId: string) => void;
	clearPendingFocus: () => void;
	setFilter: (filter: TunnelFilter) => void;
	setHelpOpen: (isOpen: boolean) => void;
	setPaletteOpen: (isOpen: boolean) => void;
	dismissTrayProblem: (kind: TrayProblemKind) => void;
	toggleTunnelSelection: (tunnelId: string) => void;
	/** Selects every tunnel between the anchor and `tunnelId` in `orderedIds`. */
	selectTunnelRange: (tunnelId: string, orderedIds: readonly string[]) => void;
	selectTunnels: (tunnelIds: readonly string[]) => void;
	/** Drops the selected tunnels that are not in `visibleIds`. */
	retainTunnelSelection: (visibleIds: readonly string[]) => void;
	clearTunnelSelection: () => void;
};

export const INITIAL_UI_STATE: UiState = {
	panel: { kind: 'none' },
	filter: { kind: 'all' },
	isHelpOpen: false,
	isPaletteOpen: false,
	dismissedTrayProblem: null,
	pendingFocusTunnelId: null,
	confirmedX11Hosts: [],
	selectedTunnelIds: new Set(),
	selectionAnchorId: null,
};

export const useUiStore = create<UiState & UiActions>()((set) => ({
	...INITIAL_UI_STATE,
	openTunnelEditor: (tunnelId) =>
		set({ panel: { kind: 'tunnelEditor', tunnelId } }),
	openTunnelImport: (draft) =>
		set({ panel: { kind: 'tunnelEditor', tunnelId: null, draft } }),
	openSettings: () => set({ panel: { kind: 'settings' } }),
	openX11: (host) => set({ panel: { kind: 'x11', host } }),
	openTunnelDetails: (tunnelId) =>
		set({ panel: { kind: 'tunnelDetails', tunnelId } }),
	openHostConfig: (host) => set({ panel: { kind: 'hostConfig', host } }),
	openImport: () => set({ panel: { kind: 'import' } }),
	confirmX11Host: (host) =>
		set((state) => ({
			confirmedX11Hosts: [...state.confirmedX11Hosts, host],
		})),
	closePanel: () => set({ panel: { kind: 'none' } }),
	closePanelFocusingTunnel: (tunnelId) =>
		set({ panel: { kind: 'none' }, pendingFocusTunnelId: tunnelId }),
	clearPendingFocus: () => set({ pendingFocusTunnelId: null }),
	setFilter: (filter) => set({ filter }),
	setHelpOpen: (isHelpOpen) => set({ isHelpOpen }),
	setPaletteOpen: (isPaletteOpen) => set({ isPaletteOpen }),
	dismissTrayProblem: (dismissedTrayProblem) => set({ dismissedTrayProblem }),
	toggleTunnelSelection: (tunnelId) =>
		set((state) => {
			const selectedTunnelIds = new Set(state.selectedTunnelIds);
			if (!selectedTunnelIds.delete(tunnelId)) selectedTunnelIds.add(tunnelId);
			return { selectedTunnelIds, selectionAnchorId: tunnelId };
		}),
	selectTunnelRange: (tunnelId, orderedIds) =>
		set((state) => ({
			selectedTunnelIds: new Set([
				...state.selectedTunnelIds,
				...getRangeIds(orderedIds, state.selectionAnchorId, tunnelId),
			]),
			selectionAnchorId: tunnelId,
		})),
	selectTunnels: (tunnelIds) =>
		set({ selectedTunnelIds: new Set(tunnelIds), selectionAnchorId: null }),
	retainTunnelSelection: (visibleIds) =>
		set((state) => {
			const visibleIdSet = new Set(visibleIds);
			const isAnchorVisible =
				state.selectionAnchorId !== null &&
				visibleIdSet.has(state.selectionAnchorId);
			return {
				selectedTunnelIds: new Set(
					[...state.selectedTunnelIds].filter((id) => visibleIdSet.has(id))
				),
				selectionAnchorId: isAnchorVisible ? state.selectionAnchorId : null,
			};
		}),
	clearTunnelSelection: () =>
		set({ selectedTunnelIds: new Set(), selectionAnchorId: null }),
}));
