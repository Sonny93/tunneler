import { describe, expect, it } from 'vitest';

import { useUiStore } from '#/state/ui_store';
import type { TunnelSpecDto } from '#/daemon/protocol';

describe('useUiStore', () => {
	it('should open the editor in create mode with a null id', () => {
		useUiStore.getState().openTunnelEditor(null);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
		});
	});

	it('should open the editor on an existing tunnel', () => {
		useUiStore.getState().openTunnelEditor('a');

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: 'a',
		});
	});

	it('should open the editor in create mode with a draft when importing', () => {
		const draft: TunnelSpecDto = {
			name: 'nova 3333',
			host: 'nova',
			kind: 'local',
			listenPort: 3333,
			tags: [],
		};

		useUiStore.getState().openTunnelImport(draft);

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelEditor',
			tunnelId: null,
			draft,
		});
	});

	it('should open the settings panel', () => {
		useUiStore.getState().openSettings();

		expect(useUiStore.getState().panel).toEqual({ kind: 'settings' });
	});

	it('should close the panel', () => {
		useUiStore.getState().openSettings();

		useUiStore.getState().closePanel();

		expect(useUiStore.getState().panel).toEqual({ kind: 'none' });
	});

	it('should close the panel and request the focus of a tunnel', () => {
		useUiStore.getState().openTunnelEditor(null);

		useUiStore.getState().closePanelFocusingTunnel('a');

		expect(useUiStore.getState()).toMatchObject({
			panel: { kind: 'none' },
			pendingFocusTunnelId: 'a',
		});
	});

	it('should clear the pending tunnel focus', () => {
		useUiStore.getState().closePanelFocusingTunnel('a');

		useUiStore.getState().clearPendingFocus();

		expect(useUiStore.getState().pendingFocusTunnelId).toBeNull();
	});

	it('should set the filter', () => {
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });

		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'dev' });
	});

	it('should toggle the help flag', () => {
		useUiStore.getState().setHelpOpen(true);

		expect(useUiStore.getState().isHelpOpen).toBe(true);
	});

	it('should open the X11 panel of a host', () => {
		useUiStore.getState().openX11('nova');

		expect(useUiStore.getState().panel).toEqual({ kind: 'x11', host: 'nova' });
	});

	it('should remember the hosts whose X11 risk was confirmed', () => {
		useUiStore.getState().confirmX11Host('nova');

		expect(useUiStore.getState().confirmedX11Hosts).toEqual(['nova']);
	});

	it('should open the details panel of a tunnel', () => {
		useUiStore.getState().openTunnelDetails('a');

		expect(useUiStore.getState().panel).toEqual({
			kind: 'tunnelDetails',
			tunnelId: 'a',
		});
	});

	it('should open the resolved configuration panel of a host', () => {
		useUiStore.getState().openHostConfig('nova');

		expect(useUiStore.getState().panel).toEqual({
			kind: 'hostConfig',
			host: 'nova',
		});
	});

	it('should open the import panel', () => {
		useUiStore.getState().openImport();

		expect(useUiStore.getState().panel).toEqual({ kind: 'import' });
	});

	it('should toggle the palette flag', () => {
		useUiStore.getState().setPaletteOpen(true);

		expect(useUiStore.getState().isPaletteOpen).toBe(true);
	});
});
