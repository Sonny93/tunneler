import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor, within } from '@testing-library/react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { buildExternalProcess } from '#/testing/factories';
import { useToastStore } from '#/shared/toasts/toast_store';
import type { ExternalProcessDto } from '#/daemon/protocol';
import { ExternalSection } from '#/externals/external_section';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { TunnelFormDrawer } from '#/tunnel_form/tunnel_form_drawer';

function renderSection(
	externals: ExternalProcessDto[] = [buildExternalProcess()],
	gateway = createFakeGateway()
) {
	act(() => useDaemonStore.setState({ externals }));
	return renderWithGateway(
		<>
			<ExternalSection />
			<TunnelFormDrawer />
			<ModalProvider />
		</>,
		gateway
	);
}

describe('ExternalSection', () => {
	it('should render nothing when there are no externals', () => {
		const { container } = renderSection([]);

		expect(container).toBeEmptyDOMElement();
	});

	it('should be a region named with the number of processes and name each process with its destination and pid', () => {
		renderSection([
			buildExternalProcess(),
			buildExternalProcess({ pid: 5, destination: undefined }),
		]);

		expect(
			screen.getByRole('region', { name: 'Unmanaged externals (2)' })
		).toBeInTheDocument();
		expect(
			screen
				.getAllByRole('heading', { level: 3 })
				.map((item) => item.textContent)
		).toEqual(['ssh to nova (pid 4242)', 'ssh (pid 5)']);
	});

	it('should list the forwards of a process', () => {
		renderSection([
			buildExternalProcess({
				forwards: [
					{ kind: 'local', listenPort: 3333, targetHost: 'localhost' },
					{ kind: 'dynamic', listenPort: 1080 },
				],
			}),
		]);

		expect(
			screen.getByText('Local forward 3333 to localhost:3333')
		).toBeInTheDocument();
		expect(screen.getByText('-D 1080')).toBeInTheDocument();
	});

	it('should offer to import every kind of forward', () => {
		renderSection([
			buildExternalProcess({
				forwards: [
					{ kind: 'local', listenPort: 3333, targetPort: 3333 },
					{ kind: 'remote', listenPort: 8080, targetPort: 80 },
					{ kind: 'dynamic', listenPort: 1080 },
				],
			}),
		]);

		expect(
			screen.getAllByRole('button', { name: /^Import as a tunnel/ })
		).toHaveLength(3);
	});

	it('should import a dynamic forward as a dynamic tunnel without target', async () => {
		const { gateway } = renderSection([
			buildExternalProcess({
				forwards: [{ kind: 'dynamic', listenPort: 1080 }],
			}),
		]);
		await userEvent.click(
			screen.getByRole('button', { name: /^Import as a tunnel/ })
		);

		await userEvent.click(
			await screen.findByRole('button', { name: 'Create' })
		);

		await waitFor(() =>
			expect(gateway.createTunnel).toHaveBeenCalledWith(
				expect.objectContaining({
					kind: 'dynamic',
					listenPort: 1080,
					targetHost: undefined,
					targetPort: undefined,
				})
			)
		);
	});

	it('should import a remote forward as a remote tunnel with its target', async () => {
		const { gateway } = renderSection([
			buildExternalProcess({
				forwards: [
					{
						kind: 'remote',
						listenPort: 8080,
						targetHost: 'app',
						targetPort: 3000,
					},
				],
			}),
		]);
		await userEvent.click(
			screen.getByRole('button', { name: /^Import as a tunnel/ })
		);

		await userEvent.click(
			await screen.findByRole('button', { name: 'Create' })
		);

		await waitFor(() =>
			expect(gateway.createTunnel).toHaveBeenCalledWith(
				expect.objectContaining({
					kind: 'remote',
					listenPort: 8080,
					targetHost: 'app',
					targetPort: 3000,
				})
			)
		);
	});

	it('should open the create drawer prefilled when importing', async () => {
		renderSection();

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Import as a tunnel: 3333 to localhost:3333',
			})
		);

		expect(await screen.findByRole('combobox', { name: /^Host/ })).toHaveValue(
			'nova'
		);
	});

	it('should create a tunnel with the kind and values of the draft', async () => {
		const { gateway } = renderSection([
			buildExternalProcess({
				forwards: [
					{
						kind: 'local',
						listenPort: 3333,
						targetHost: 'db',
						targetPort: 5432,
					},
				],
			}),
		]);
		await userEvent.click(
			screen.getByRole('button', { name: /^Import as a tunnel/ })
		);

		await userEvent.click(
			await screen.findByRole('button', { name: 'Create' })
		);

		await waitFor(() =>
			expect(gateway.createTunnel).toHaveBeenCalledWith({
				host: 'nova',
				kind: 'local',
				listenPort: 3333,
				listenAddress: undefined,
				targetHost: 'db',
				targetPort: 5432,
				tags: [],
			})
		);
		expect(useUiStore.getState().panel.kind).toBe('none');
	});

	it('should ask for confirmation then kill the process', async () => {
		const { gateway } = renderSection();
		await userEvent.click(
			screen.getByRole('button', {
				name: 'Stop the process: ssh (pid 4242)',
			})
		);

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Stop ssh (pid 4242)?',
		});
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Stop the process' })
		);

		expect(gateway.killExternal).toHaveBeenCalledWith(4242);
	});

	it('should not kill the process when cancelled', async () => {
		const { gateway } = renderSection();
		await userEvent.click(
			screen.getByRole('button', {
				name: 'Stop the process: ssh (pid 4242)',
			})
		);

		await userEvent.click(
			await screen.findByRole('button', { name: 'Cancel' })
		);

		expect(gateway.killExternal).not.toHaveBeenCalled();
	});

	it('should ignore the process and confirm with a toast', async () => {
		const { gateway } = renderSection();

		await userEvent.click(
			screen.getByRole('button', { name: 'Ignore ssh (pid 4242)' })
		);

		expect(gateway.ignoreExternal).toHaveBeenCalledWith(4242);
		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				'Process ssh (pid 4242) ignored'
			)
		);
	});

	it('should have no accessibility violation', async () => {
		const { container } = renderSection([
			buildExternalProcess({
				forwards: [
					{ kind: 'local', listenPort: 3333, targetPort: 3333 },
					{ kind: 'dynamic', listenPort: 1080 },
				],
			}),
		]);

		expect(await axe(container)).toHaveNoViolations();
	});
});
