import { describe, expect, it } from 'vitest';

import type { TunnelSpecDto } from '#/daemon/protocol';
import { useUiStore } from '#/state/ui_store';

describe('useUiStore', () => {
	it('should toggle the sidebar and remember it', () => {
		useUiStore.getState().toggleSidebar();

		expect(useUiStore.getState().isSidebarCollapsed).toBe(true);
		expect(localStorage.getItem('tunneler.sidebarCollapsed')).toBe('true');
	});

	it('should expand the sidebar again and forget the collapse', () => {
		useUiStore.getState().toggleSidebar();
		useUiStore.getState().toggleSidebar();

		expect(useUiStore.getState().isSidebarCollapsed).toBe(false);
		expect(localStorage.getItem('tunneler.sidebarCollapsed')).toBeNull();
	});

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

describe('useUiStore tunnel selection', () => {
	const selectedIds = () => [...useUiStore.getState().selectedTunnelIds];

	it('should select a tunnel when toggled the first time', () => {
		useUiStore.getState().toggleTunnelSelection('a');

		expect(selectedIds()).toEqual(['a']);
	});

	it('should unselect a tunnel when toggled again', () => {
		useUiStore.getState().toggleTunnelSelection('a');
		useUiStore.getState().toggleTunnelSelection('a');

		expect(selectedIds()).toEqual([]);
	});

	it('should select the range from the last toggled tunnel', () => {
		useUiStore.getState().toggleTunnelSelection('b');

		useUiStore.getState().selectTunnelRange('d', ['a', 'b', 'c', 'd']);

		expect(selectedIds()).toEqual(['b', 'c', 'd']);
	});

	it('should keep the previous selection when selecting a range', () => {
		useUiStore.getState().toggleTunnelSelection('a');
		useUiStore.getState().toggleTunnelSelection('c');

		useUiStore.getState().selectTunnelRange('d', ['a', 'b', 'c', 'd']);

		expect(selectedIds()).toEqual(['a', 'c', 'd']);
	});

	it('should select every given tunnel', () => {
		useUiStore.getState().selectTunnels(['a', 'b']);

		expect(selectedIds()).toEqual(['a', 'b']);
	});

	it('should drop the tunnels that are no longer visible', () => {
		useUiStore.getState().selectTunnels(['a', 'b', 'c']);

		useUiStore.getState().retainTunnelSelection(['b', 'c', 'd']);

		expect(selectedIds()).toEqual(['b', 'c']);
	});

	it('should forget the anchor when it is no longer visible', () => {
		useUiStore.getState().toggleTunnelSelection('a');

		useUiStore.getState().retainTunnelSelection(['b']);

		expect(useUiStore.getState().selectionAnchorId).toBeNull();
	});

	it('should clear the selection and the anchor', () => {
		useUiStore.getState().toggleTunnelSelection('a');

		useUiStore.getState().clearTunnelSelection();

		expect(selectedIds()).toEqual([]);
		expect(useUiStore.getState().selectionAnchorId).toBeNull();
	});
});
