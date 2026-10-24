import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { useToastStore } from '#/shared/toasts/toast_store';
import { buildMetrics, buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelRow } from '#/tunnels/list/tunnel_row';
import { TunnelSelectionContext } from '#/tunnels/selection/tunnel_selection_context';

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
	it('should name the switch after the endpoint and state', () => {
		renderRow({ state: { kind: 'active' } });

		expect(
			screen.getByRole('switch', {
				name: 'Tunnel 3333 to nova:3333, active',
			})
		).toBeChecked();
	});

	it('should leave the switch unchecked when stopped', () => {
		renderRow();

		expect(screen.getByRole('switch')).not.toBeChecked();
	});

	it('should show the endpoint, the state and the tags', () => {
		renderRow({ tags: ['dev'] });

		expect(screen.getByText('3333 → nova:3333')).toBeInTheDocument();
		expect(screen.getByText('Stopped')).toBeInTheDocument();
		expect(screen.getByText('dev')).toBeInTheDocument();
	});

	it('should offer an unlabeled checkbox named after the endpoint', () => {
		renderRow();

		expect(
			screen.getByRole('checkbox', { name: 'Select 3333 to nova:3333' })
		).not.toBeChecked();
	});

	it('should check the checkbox of a selected tunnel', () => {
		renderWithGateway(
			<TunnelSelectionContext
				value={{
					selectedIds: new Set(['tunnel-1']),
					selectedTunnels: [buildTunnel()],
					toggle: () => undefined,
					selectRange: () => undefined,
				}}
			>
				<ul>
					<TunnelRow tunnel={buildTunnel()} isActiveRow />
				</ul>
			</TunnelSelectionContext>
		);

		expect(
			screen.getByRole('checkbox', { name: 'Select 3333 to nova:3333' })
		).toBeChecked();
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
			"Couldn't start the tunnel"
		);
	});

	it('should push an error toast when stopping fails', async () => {
		const gateway = createFakeGateway();
		gateway.stopTunnel.mockRejectedValue(new Error('boom'));
		renderRow({ state: { kind: 'active' } }, gateway);

		await userEvent.click(screen.getByRole('switch'));

		expect(useToastStore.getState().toasts[0]?.message).toContain(
			"Couldn't stop the tunnel"
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
			screen.getByText(/2 connections, .* received, .* sent/)
		).toBeInTheDocument();
	});

	it('should not show traffic for a stopped tunnel', () => {
		renderRow();

		expect(screen.queryByText(/connection/)).not.toBeInTheDocument();
	});

	it('should read the endpoint with to instead of the arrow', () => {
		renderRow();

		expect(screen.getAllByText('3333 to nova:3333').length).toBeGreaterThan(0);
	});

	it('should describe the failure reason on the switch when the tunnel failed', () => {
		renderRow({
			state: { kind: 'failed', reason: { kind: 'connect', message: 'boom' } },
		});

		expect(screen.getByRole('switch')).toHaveAccessibleDescription(
			'SSH connection failed: boom'
		);
	});

	it('should keep the whole health error in the row and truncate its rendering', () => {
		const error = 'dial tcp 198.51.100.10:5432: connect: connection timed out';
		renderRow({
			state: { kind: 'active' },
			health: { kind: 'unreachable', since: 0, error },
		});

		expect(screen.getAllByText(error).at(-1)).toHaveClass('truncate');
	});

	it('should not open a tooltip when the truncated health error is hovered', async () => {
		const error = 'dial tcp 198.51.100.10:5432: connect: connection timed out';
		const user = userEvent.setup();
		renderRow({
			state: { kind: 'active' },
			health: { kind: 'unreachable', since: 0, error },
		});

		await user.hover(screen.getAllByText(error).at(-1) as HTMLElement);

		expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
	});

	it('should have no axe violations when the health error is truncated', async () => {
		const { container } = renderRow({
			state: { kind: 'active' },
			health: { kind: 'unreachable', since: 0, error: 'timeout' },
		});

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should describe the owner on the switch when the tunnel is external', () => {
		renderRow({
			state: { kind: 'external', owner: { pid: 4242, process: 'ssh' } },
		});

		expect(screen.getByRole('switch')).toHaveAccessibleDescription(
			'provided by ssh (pid 4242)'
		);
	});

	it('should describe the holder on the switch when the tunnel is in conflict', () => {
		renderRow({ state: { kind: 'conflict' } });

		expect(screen.getByRole('switch')).toHaveAccessibleDescription(
			'port in use by an unknown process'
		);
	});

	it('should describe the health error on the switch when the target is unreachable', () => {
		renderRow({
			state: { kind: 'active' },
			health: { kind: 'unreachable', since: 0, error: 'timeout' },
		});

		expect(screen.getByRole('switch')).toHaveAccessibleDescription('timeout');
	});

	it('should not describe the switch when the tunnel has no problem', () => {
		renderRow({ state: { kind: 'active' } });

		expect(screen.getByRole('switch')).not.toHaveAttribute('aria-describedby');
	});

	it('should open a running local tunnel with its default action when its row is double-clicked', async () => {
		const { gateway } = renderRow({ state: { kind: 'active' } });

		await userEvent.dblClick(screen.getByText('3333 → nova:3333'));

		expect(gateway.openTunnel).toHaveBeenCalledWith('tunnel-1', undefined);
	});

	it('should not open a stopped tunnel when its row is double-clicked', async () => {
		const { gateway } = renderRow();

		await userEvent.dblClick(screen.getByText('3333 → nova:3333'));

		expect(gateway.openTunnel).not.toHaveBeenCalled();
	});

	it('should not open the tunnel when its switch is double-clicked', async () => {
		const { gateway } = renderRow({ state: { kind: 'active' } });

		await userEvent.dblClick(screen.getByRole('switch'));

		expect(gateway.openTunnel).not.toHaveBeenCalled();
	});

	it('should not open a remote tunnel when its row is double-clicked', async () => {
		const { gateway } = renderRow({
			kind: 'remote',
			state: { kind: 'active' },
		});

		await userEvent.dblClick(screen.getByText('Active'));

		expect(gateway.openTunnel).not.toHaveBeenCalled();
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
