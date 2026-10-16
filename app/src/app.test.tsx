import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { screen, within } from '@testing-library/react';

import { App } from '#/app';
import { createFakeGateway } from '#/testing/fake_gateway';
import { buildSnapshot, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';

const renderConnectedApp = async () => {
	const rendered = renderWithGateway(<App />);
	await screen.findByRole('navigation', { name: 'Filtres des tunnels' });
	return rendered;
};

const renderDisconnectedApp = async () => {
	const gateway = createFakeGateway();
	gateway.connect.mockRejectedValue({
		code: 'daemon_unreachable',
		message: 'introuvable',
	});
	const rendered = renderWithGateway(<App />, gateway);
	await screen.findByText(/Daemon injoignable/);
	return rendered;
};

describe('App', () => {
	it('should show a status message while connecting', () => {
		const gateway = createFakeGateway();
		gateway.connect.mockReturnValue(new Promise(() => undefined));

		renderWithGateway(<App />, gateway);

		expect(
			within(screen.getByRole('main')).getByRole('status')
		).toHaveTextContent('Connexion au daemon…');
	});

	it('should have no axe violations while connecting', async () => {
		const gateway = createFakeGateway();
		gateway.connect.mockReturnValue(new Promise(() => undefined));
		const { container } = renderWithGateway(<App />, gateway);

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should render the main landmark once connected', async () => {
		await renderConnectedApp();

		expect(await screen.findByRole('main')).toBeInTheDocument();
	});

	it('should render the app title once connected', async () => {
		await renderConnectedApp();

		expect(
			screen.getByRole('heading', { name: 'Tunneler', level: 1 })
		).toBeInTheDocument();
	});

	it('should list the tunnels of the snapshot once connected', async () => {
		const gateway = createFakeGateway({
			snapshot: buildSnapshot({ tunnels: [buildTunnel({ name: 'Adonis' })] }),
		});
		renderWithGateway(<App />, gateway);

		expect(
			await screen.findByRole('switch', { name: /^Tunnel Adonis,/ })
		).toBeInTheDocument();
	});

	it('should have no axe violations once connected', async () => {
		const { container } = await renderConnectedApp();

		expect(await axe(container)).toHaveNoViolations();
	});

	it('should show the daemon error in an alert when disconnected', async () => {
		await renderDisconnectedApp();

		expect(
			screen.getByText('Daemon injoignable : introuvable')
		).toBeInTheDocument();
	});

	it('should connect again when the retry button is pressed', async () => {
		const user = userEvent.setup();
		const { gateway } = await renderDisconnectedApp();

		await user.click(screen.getByRole('button', { name: 'Réessayer' }));

		expect(gateway.connect).toHaveBeenCalledTimes(2);
	});

	it('should have no axe violations when disconnected', async () => {
		const { container } = await renderDisconnectedApp();

		expect(await axe(container)).toHaveNoViolations();
	});
});
