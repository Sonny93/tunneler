import { describe, expect, it } from 'vitest';
import { screen, waitFor } from '@testing-library/react';

import { App } from '#/app';
import { useUpdateStore } from '#/updates/update_store';
import type { UpdateOfferDto } from '#/daemon/protocol';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { buildSettings, buildSnapshot } from '#/testing/factories';

const OFFER: UpdateOfferDto = {
	version: '0.2.0',
	releaseUrl: 'https://github.com/Sonny93/tunneler/releases/tag/v0.2.0',
	action: 'releaseLinkOnly',
};

const buildGateway = (checkForUpdates: boolean) => {
	const gateway = createFakeGateway({
		snapshot: buildSnapshot({ settings: buildSettings({ checkForUpdates }) }),
	});
	gateway.checkForUpdate.mockResolvedValue(OFFER);
	return gateway;
};

describe('useUpdateCheck', () => {
	it('should check once and show the banner when the setting is on', async () => {
		const gateway = buildGateway(true);

		renderWithGateway(<App />, gateway);

		expect(
			await screen.findByRole('region', { name: 'Version 0.2.0 disponible' })
		).toBeInTheDocument();
		expect(gateway.checkForUpdate).toHaveBeenCalledOnce();
	});

	it('should not check when the setting is off', async () => {
		const gateway = buildGateway(false);

		renderWithGateway(<App />, gateway);

		await screen.findByRole('navigation', { name: 'Filtres des tunnels' });
		await waitFor(() =>
			expect(useUpdateStore.getState().hasChecked).toBe(true)
		);
		expect(gateway.checkForUpdate).not.toHaveBeenCalled();
	});

	it('should show no banner when the check finds nothing', async () => {
		const gateway = buildGateway(true);
		gateway.checkForUpdate.mockResolvedValue(null);

		renderWithGateway(<App />, gateway);

		await waitFor(() => expect(gateway.checkForUpdate).toHaveBeenCalled());
		expect(
			screen.queryByRole('region', { name: /disponible/ })
		).not.toBeInTheDocument();
	});

	it('should show no message when the check itself fails', async () => {
		const gateway = buildGateway(true);
		gateway.checkForUpdate.mockRejectedValue(new Error('hors ligne'));

		renderWithGateway(<App />, gateway);

		await waitFor(() => expect(gateway.checkForUpdate).toHaveBeenCalled());
		expect(
			screen.queryByRole('region', { name: /disponible/ })
		).not.toBeInTheDocument();
		expect(screen.queryByRole('alert', { name: /./ })).not.toBeInTheDocument();
	});
});
