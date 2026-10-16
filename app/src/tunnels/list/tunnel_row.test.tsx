import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { TunnelRow } from '#/tunnels/list/tunnel_row';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildMetrics, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';

function renderRow(
	overrides: Parameters<typeof buildTunnel>[0] = {},
	gateway = createFakeGateway()
) {
	return renderWithGateway(
		<ul>
			<TunnelRow tunnel={buildTunnel(overrides)} isActiveRow />
		</ul>,
		gateway
	);
}

describe('TunnelRow', () => {
	it('should name the switch after the tunnel, endpoint and state', () => {
		renderRow({ state: { kind: 'active' } });

		expect(
			screen.getByRole('switch', {
				name: 'Tunnel Adonis, 3333 vers nova:3333, actif',
			})
		).toBeChecked();
	});

	it('should leave the switch unchecked when stopped', () => {
		renderRow();

		expect(screen.getByRole('switch')).not.toBeChecked();
	});

	it('should show the name, the state and the tags', () => {
		renderRow({ tags: ['dev'] });

		expect(screen.getByText('Adonis')).toBeInTheDocument();
		expect(screen.getByText('Arrêté')).toBeInTheDocument();
		expect(screen.getByText('dev')).toBeInTheDocument();
	});

	it('should start a stopped tunnel when toggled', async () => {
		const { gateway } = renderRow();

		await userEvent.click(screen.getByRole('switch'));

		expect(gateway.startTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should start a failed tunnel again when toggled', async () => {
		const { gateway } = renderRow({
			state: { kind: 'failed', reason: { kind: 'connect', message: 'x' } },
		});

		await userEvent.click(screen.getByRole('switch'));

		expect(gateway.startTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should stop a running tunnel when toggled', async () => {
		const { gateway } = renderRow({ state: { kind: 'active' } });

		await userEvent.click(screen.getByRole('switch'));

		expect(gateway.stopTunnel).toHaveBeenCalledWith('tunnel-1');
	});

	it('should push an error toast when starting fails', async () => {
		const gateway = createFakeGateway();
		gateway.startTunnel.mockRejectedValue(new Error('boom'));
		renderRow({}, gateway);

		await userEvent.click(screen.getByRole('switch'));

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			'Démarrage impossible'
		);
	});

	it('should push an error toast when stopping fails', async () => {
		const gateway = createFakeGateway();
		gateway.stopTunnel.mockRejectedValue(new Error('boom'));
		renderRow({ state: { kind: 'active' } }, gateway);

		await userEvent.click(screen.getByRole('switch'));

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			'Arrêt impossible'
		);
	});

	it('should describe the traffic of a running tunnel in words', () => {
		renderRow({
			state: { kind: 'active' },
			metrics: buildMetrics({
				activeConnections: 2,
				bytesIn: 2048,
				bytesOut: 512,
			}),
		});

		expect(
			screen.getByText(/2 connexions, .* reçus, .* envoyés/)
		).toBeInTheDocument();
	});

	it('should not show traffic for a stopped tunnel', () => {
		renderRow();

		expect(screen.queryByText(/connexion/)).not.toBeInTheDocument();
	});

	it('should read the endpoint with vers instead of the arrow', () => {
		renderRow();

		expect(screen.getAllByText('3333 vers nova:3333').length).toBeGreaterThan(
			0
		);
	});

	it('should have no accessibility violation', async () => {
		const { container } = renderRow({
			tags: ['dev'],
			state: { kind: 'active' },
			health: { kind: 'reachable' },
		});

		expect(await axe(container)).toHaveNoViolations();
	});
});
