import { create } from 'zustand';

import type { EventMessage, SettingsDto, SnapshotDto } from '#/daemon/protocol';
import {
	INITIAL_DAEMON_STATE,
	type ConnectionState,
	type DaemonState,
} from '#/state/daemon_state';
import { getPromptId } from '#/state/prompt_id';
import { reduceDaemonEvent } from '#/state/reduce_daemon_event';

type DaemonActions = {
	setConnection: (connection: ConnectionState) => void;
	applySnapshot: (snapshot: SnapshotDto) => void;
	setSettings: (settings: SettingsDto) => void;
	setHosts: (hosts: readonly string[]) => void;
	applyEvent: (event: EventMessage) => void;
	dismissPrompt: (id: string) => void;
	clearQuitRequest: () => void;
	clearStandbyRequest: () => void;
};

export const useDaemonStore = create<DaemonState & DaemonActions>()((set) => ({
	...INITIAL_DAEMON_STATE,
	setConnection: (connection) => set({ connection }),
	applySnapshot: (snapshot) =>
		set({
			tunnels: snapshot.tunnels,
			externals: snapshot.externals,
			x11Anchors: snapshot.x11Anchors,
			x11Launchers: snapshot.x11Launchers,
			openers: snapshot.openers,
			settings: snapshot.settings,
			tray: snapshot.tray,
			configErrors: [],
		}),
	setSettings: (settings) => set({ settings }),
	setHosts: (hosts) => set({ hosts }),
	applyEvent: (event) => set((state) => reduceDaemonEvent(state, event)),
	dismissPrompt: (id) =>
		set((state) => ({
			prompts: state.prompts.filter((prompt) => getPromptId(prompt) !== id),
		})),
	clearQuitRequest: () => set({ isQuitRequested: false }),
	clearStandbyRequest: () => set({ isStandbyRequested: false }),
}));
