import { act, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import type { UpdateOfferDto } from '#/daemon/protocol';
import { toSettingsFormValues } from '#/settings/settings_form_values';
import { SettingsUpdatesFieldset } from '#/settings/settings_updates_fieldset';
import { buildSettings } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { useUpdateStore } from '#/updates/update_store';

const OFFER: UpdateOfferDto = {
	version: '0.3.0',
	releaseUrl: 'https://github.com/Sonny93/tunneler/releases/tag/v0.3.0',
	action: 'releaseLinkOnly',
};

const renderFieldset = (gateway = createFakeGateway()) =>
	renderWithGateway(
		<SettingsUpdatesFieldset
			values={toSettingsFormValues(buildSettings())}
			onChangeToggle={() => undefined}
		/>,
		gateway
	);

const clickCheckNow = () =>
	userEvent.click(screen.getByRole('button', { name: 'Check now' }));

describe('SettingsUpdateCheck', () => {
	it('should have an empty status region before any check and no axe violations', async () => {
		const { container } = renderFieldset();

		expect(screen.getByRole('status')).toBeEmptyDOMElement();
		expect(await axe(container)).toHaveNoViolations();
	});

	it('should say the version is the latest when nothing newer exists', async () => {
		const { gateway } = renderFieldset();

		await clickCheckNow();

		expect(await screen.findByText('You have the latest version')).toBe(
			screen.getByRole('status')
		);
		expect(gateway.checkForUpdate).toHaveBeenCalledOnce();
	});

	it('should store the offer and show the hidden banner again when a version exists', async () => {
		const gateway = createFakeGateway();
		gateway.checkForUpdate.mockResolvedValue(OFFER);
		useUpdateStore.setState({ isHidden: true });
		renderFieldset(gateway);

		await clickCheckNow();

		expect(await screen.findByRole('status')).toHaveTextContent(
			'Version 0.3.0 is available'
		);
		expect(useUpdateStore.getState().offer).toEqual(OFFER);
		expect(useUpdateStore.getState().isHidden).toBe(false);
	});

	it('should say the check failed and keep the stored offer', async () => {
		const gateway = createFakeGateway();
		gateway.checkForUpdate.mockRejectedValue(new Error('offline'));
		useUpdateStore.setState({ offer: OFFER });
		renderFieldset(gateway);

		await clickCheckNow();

		expect(await screen.findByRole('status')).toHaveTextContent(
			"Couldn't check for updates"
		);
		expect(useUpdateStore.getState().offer).toEqual(OFFER);
	});

	it('should be busy and check once while a check is running', async () => {
		const gateway = createFakeGateway();
		let finishCheck: (offer: UpdateOfferDto | null) => void = () => undefined;
		gateway.checkForUpdate.mockReturnValue(
			new Promise((resolve) => {
				finishCheck = resolve;
			})
		);
		renderFieldset(gateway);

		await clickCheckNow();
		await clickCheckNow();

		expect(screen.getByRole('status')).toHaveTextContent(
			'Checking for updates…'
		);
		expect(screen.getByRole('button', { name: /Check now/ })).toHaveAttribute(
			'aria-busy',
			'true'
		);
		expect(gateway.checkForUpdate).toHaveBeenCalledOnce();
		await act(async () => finishCheck(null));
		expect(screen.getByRole('status')).toHaveTextContent(
			'You have the latest version'
		);
	});

	it('should not check while an installation is running', async () => {
		useUpdateStore.setState({ isInstalling: true });
		const { gateway } = renderFieldset();

		await clickCheckNow();

		expect(gateway.checkForUpdate).not.toHaveBeenCalled();
		expect(screen.getByRole('status')).toBeEmptyDOMElement();
	});
});
