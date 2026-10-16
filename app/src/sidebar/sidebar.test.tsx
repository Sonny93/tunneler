import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, within } from '@testing-library/react';

import { Sidebar } from '#/sidebar/sidebar';
import { useUiStore } from '#/state/ui_store';
import { buildTunnel } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';

const ACTIVE = { kind: 'active' } as const;

function seedTunnels() {
	useDaemonStore.setState({
		tunnels: [
			buildTunnel({ id: 'a', host: 'nova', tags: ['dev'], state: ACTIVE }),
			buildTunnel({ id: 'b', host: 'nova', tags: ['dev', 'Db'] }),
			buildTunnel({ id: 'c', host: 'zeta', tags: [], state: ACTIVE }),
		],
	});
}

describe('Sidebar', () => {
	it('should expose a labelled navigation', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('navigation', { name: 'Filtres des tunnels' })
		).toBeInTheDocument();
	});

	it('should show the number of running tunnels on the all filter', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		).toHaveTextContent('2/3 actifs');
	});

	it('should read the active count in words in the button name', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('button', {
				name: /^Tous les tunnels\s?2 sur 3 actifs$/,
			})
		).toBeInTheDocument();
	});

	it('should list tags sorted ignoring case with their counts', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		const tagList = within(
			screen.getByRole('heading', { name: 'Tags' }).parentElement as HTMLElement
		);

		expect(
			tagList.getAllByRole('button').map((button) => button.textContent)
		).toEqual(['Db0/1 actifs0 sur 1 actifs', 'dev1/2 actifs1 sur 2 actifs']);
	});

	it('should list the hosts used by tunnels', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		expect(screen.getByRole('button', { name: /^nova/ })).toHaveTextContent(
			'1/2 actifs'
		);
	});

	it('should hide the tags section when no tunnel has a tag', () => {
		useDaemonStore.setState({ tunnels: [buildTunnel()] });
		renderWithGateway(<Sidebar />);

		expect(
			screen.queryByRole('heading', { name: 'Tags' })
		).not.toBeInTheDocument();
	});

	it('should mark the all filter as current by default', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		).toHaveAttribute('aria-current', 'true');
	});

	it('should filter by tag when its button is clicked', async () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		await userEvent.click(screen.getByRole('button', { name: /^dev/ }));

		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'dev' });
		expect(screen.getByRole('button', { name: /^dev/ })).toHaveAttribute(
			'aria-current',
			'true'
		);
	});

	it('should filter by host when its button is clicked', async () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		await userEvent.click(screen.getByRole('button', { name: /^zeta/ }));

		expect(useUiStore.getState().filter).toEqual({
			kind: 'host',
			host: 'zeta',
		});
	});

	it('should go back to all tunnels from the all filter', async () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'host', host: 'zeta' } });
		renderWithGateway(<Sidebar />);

		await userEvent.click(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		);

		expect(useUiStore.getState().filter).toEqual({ kind: 'all' });
	});

	it('should mark all as current when the filtered tag disappeared', () => {
		seedTunnels();
		useUiStore.setState({ filter: { kind: 'tag', tag: 'gone' } });
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('button', { name: /Tous les tunnels/ })
		).toHaveAttribute('aria-current', 'true');
	});

	it('should name the tag switch and describe its count', () => {
		seedTunnels();
		renderWithGateway(<Sidebar />);

		expect(
			screen.getByRole('switch', { name: 'Tous les tunnels du tag dev' })
		).toHaveAccessibleDescription('1 sur 2 actifs');
	});

	it('should start the whole tag when not every tunnel runs', async () => {
		seedTunnels();
		const { gateway } = renderWithGateway(<Sidebar />);

		await userEvent.click(
			screen.getByRole('switch', { name: 'Tous les tunnels du tag dev' })
		);

		expect(gateway.startTag).toHaveBeenCalledWith('dev');
	});

	it('should stop the whole tag when every tunnel runs', async () => {
		useDaemonStore.setState({
			tunnels: [buildTunnel({ tags: ['dev'], state: ACTIVE })],
		});
		const { gateway } = renderWithGateway(<Sidebar />);
		const tagSwitch = screen.getByRole('switch', {
			name: 'Tous les tunnels du tag dev',
		});

		await userEvent.click(tagSwitch);

		expect(tagSwitch).toBeChecked();
		expect(gateway.stopTag).toHaveBeenCalledWith('dev');
	});

	it('should push an error toast when starting a tag fails', async () => {
		seedTunnels();
		const gateway = createFakeGateway();
		gateway.startTag.mockRejectedValue(new Error('boom'));
		renderWithGateway(<Sidebar />, gateway);

		await userEvent.click(
			screen.getByRole('switch', { name: 'Tous les tunnels du tag dev' })
		);

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			'Démarrage impossible'
		);
	});

	it('should have no accessibility violation', async () => {
		seedTunnels();
		const { container } = renderWithGateway(<Sidebar />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
