import { ModalProvider } from '@minimalstuff/ui';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { DevslotLinkDto } from '#/daemon/protocol';
import { DevslotLinkHost } from '#/devslot_link/devslot_link_host';
import { useDevslotLinkListener } from '#/devslot_link/use_devslot_link_listener';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { buildDevslotUpResult } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { findFilledLiveRegion } from '#/testing/filled_live_region';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { getShownToasts } from '#/testing/shown_toasts';

vi.mock('#/devslot_link/use_activation_delay', () => ({
	useActivationDelay: () => true,
}));

const VALID_LINK: DevslotLinkDto = {
	kind: 'devslotUp',
	host: 'vega',
	instance: 'web-shop-098a',
};

function Harness() {
	useDevslotLinkListener();
	return <DevslotLinkHost />;
}

function renderHost(gateway = createFakeGateway(), hosts = ['vega']) {
	useDaemonStore.setState({ hosts });
	return renderWithGateway(
		<>
			<ModalProvider />
			<Harness />
		</>,
		gateway
	);
}

const offer = (link: DevslotLinkDto) =>
	act(() => void useUiStore.getState().offerDevslotLink(link));

describe('DevslotLinkHost', () => {
	it('should ask for confirmation without calling the daemon', async () => {
		const { gateway } = renderHost();

		offer(VALID_LINK);

		const dialog = await screen.findByRole('alertdialog', {
			name: 'Open a devslot instance',
		});
		expect(dialog).toHaveTextContent('web-shop-098a');
		expect(dialog).toHaveTextContent('vega');
		expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
		expect(gateway.devslotUp).not.toHaveBeenCalled();
	});

	it('should open the instance, toast and close when confirmed', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHost();
		offer(VALID_LINK);

		await user.click(await screen.findByRole('button', { name: /^Open/ }));

		await waitFor(() =>
			expect(gateway.devslotUp).toHaveBeenCalledWith(
				'vega',
				'web-shop-098a',
				true
			)
		);
		await waitFor(() =>
			expect(getShownToasts()).toEqual([
				expect.objectContaining({ tone: 'success' }),
			])
		);
		expect(useUiStore.getState().devslotLinkRequest).toBeNull();
	});

	it('should do nothing on Cancel', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHost();
		offer(VALID_LINK);

		await user.click(await screen.findByRole('button', { name: 'Cancel' }));

		await waitFor(() =>
			expect(useUiStore.getState().devslotLinkRequest).toBeNull()
		);
		expect(gateway.devslotUp).not.toHaveBeenCalled();
	});

	it('should do nothing on Escape', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHost();
		offer(VALID_LINK);
		await screen.findByRole('alertdialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(useUiStore.getState().devslotLinkRequest).toBeNull()
		);
		expect(gateway.devslotUp).not.toHaveBeenCalled();
	});

	it('should show a busy state while the call runs', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.devslotUp.mockReturnValue(new Promise(() => undefined));
		renderHost(gateway);
		offer(VALID_LINK);

		await user.click(await screen.findByRole('button', { name: /^Open/ }));

		expect(screen.getByRole('button', { name: /^Open/ })).toHaveAttribute(
			'aria-busy',
			'true'
		);
		expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled();
		expect(await axe(screen.getByRole('alertdialog'))).toHaveNoViolations();
	});

	it('should toast the outcome when the dialog was closed during the call', async () => {
		const user = userEvent.setup();
		let resolveCall: (
			value: ReturnType<typeof buildDevslotUpResult>
		) => void = () => undefined;
		const gateway = createFakeGateway();
		gateway.devslotUp.mockReturnValue(
			new Promise((resolve) => (resolveCall = resolve))
		);
		renderHost(gateway);
		offer(VALID_LINK);
		await user.click(await screen.findByRole('button', { name: /^Open/ }));

		await user.keyboard('{Escape}');
		await waitFor(() =>
			expect(useUiStore.getState().devslotLinkRequest).toBeNull()
		);
		act(() => resolveCall(buildDevslotUpResult()));

		await waitFor(() => expect(getShownToasts()).toHaveLength(1));
	});

	it('should lead the error with a sentence that names the problem', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.devslotUp.mockRejectedValue(
			new DaemonCommandError('internal', 'x')
		);
		renderHost(gateway);
		offer(VALID_LINK);

		await user.click(await screen.findByRole('button', { name: /^Open/ }));

		expect(await findFilledLiveRegion('alert')).toHaveTextContent(
			"Couldn't open web-shop-098a on vega."
		);
	});

	it('should keep the toast of a closed dialog with problems until dismissed', async () => {
		const user = userEvent.setup();
		let resolveCall: (
			value: ReturnType<typeof buildDevslotUpResult>
		) => void = () => undefined;
		const gateway = createFakeGateway();
		gateway.devslotUp.mockReturnValue(
			new Promise((resolve) => (resolveCall = resolve))
		);
		renderHost(gateway);
		offer(VALID_LINK);
		await user.click(await screen.findByRole('button', { name: /^Open/ }));
		await user.keyboard('{Escape}');
		await waitFor(() =>
			expect(useUiStore.getState().devslotLinkRequest).toBeNull()
		);

		act(() =>
			resolveCall(
				buildDevslotUpResult({ failed: [{ remotePort: 1, message: 'no' }] })
			)
		);

		await waitFor(() =>
			expect(getShownToasts()).toEqual([
				expect.objectContaining({ tone: 'warning' }),
			])
		);
	});

	it('should show an error in the dialog with a Close button', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.devslotUp.mockRejectedValue(
			new DaemonCommandError('devslot_instance_not_found', 'raw', {
				host: 'vega',
				instance: 'web-shop-098a',
			})
		);
		renderHost(gateway);
		offer(VALID_LINK);

		await user.click(await screen.findByRole('button', { name: /^Open/ }));

		expect(await findFilledLiveRegion('alert')).toHaveTextContent(
			'vega does not list the devslot instance web-shop-098a'
		);
		expect(document.activeElement).toHaveTextContent('Close');
		expect(getShownToasts()).toEqual([]);
		expect(await axe(screen.getByRole('alertdialog'))).toHaveNoViolations();
	});

	it('should keep partial failures in the dialog', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.devslotUp.mockResolvedValue(
			buildDevslotUpResult({
				failed: [{ remotePort: 20051, message: 'refused' }],
			})
		);
		renderHost(gateway);
		offer(VALID_LINK);

		await user.click(await screen.findByRole('button', { name: /^Open/ }));

		expect(await findFilledLiveRegion('alert')).toHaveTextContent(
			'Port 20051 could not be started: refused'
		);
		expect(getShownToasts()).toEqual([]);
	});

	it('should explain an invalid link with only a Close button', async () => {
		renderHost();

		offer({ kind: 'invalid' });

		const dialog = await screen.findByRole('alertdialog', {
			name: 'This link cannot be opened',
		});
		expect(dialog).toHaveTextContent('not a valid Tunneler devslot link');
		expect(
			screen.queryByRole('button', { name: /^Open/ })
		).not.toBeInTheDocument();
		expect(await axe(dialog)).toHaveNoViolations();
	});

	it('should name an unknown alias without offering to open', async () => {
		const { gateway } = renderHost(createFakeGateway(), ['nova']);

		offer(VALID_LINK);

		const dialog = await screen.findByRole('alertdialog');
		expect(dialog).toHaveTextContent(
			'The host vega is not in your SSH configuration'
		);
		expect(
			screen.queryByRole('button', { name: /^Open/ })
		).not.toBeInTheDocument();
		expect(gateway.devslotUp).not.toHaveBeenCalled();
		expect(await axe(dialog)).toHaveNoViolations();
	});

	it('should have no axe violations on the confirmation', async () => {
		renderHost();
		offer(VALID_LINK);

		expect(
			await axe(await screen.findByRole('alertdialog'))
		).toHaveNoViolations();
	});
});

describe('useDevslotLinkListener', () => {
	it('should toast an error when the launch link cannot be read', async () => {
		const gateway = createFakeGateway();
		gateway.takeLaunchLink.mockRejectedValue(
			new DaemonCommandError('internal', 'x')
		);
		renderHost(gateway);

		await waitFor(() =>
			expect(getShownToasts()).toEqual([
				expect.objectContaining({ tone: 'error' }),
			])
		);
	});

	it('should take the launch link once', async () => {
		const gateway = createFakeGateway();
		gateway.takeLaunchLink.mockResolvedValueOnce(VALID_LINK);
		renderHost(gateway);

		expect(await screen.findByRole('alertdialog')).toBeInTheDocument();
		expect(gateway.takeLaunchLink).toHaveBeenCalled();
	});

	it('should open the dialog of a relayed link', async () => {
		const gateway = createFakeGateway();
		renderHost(gateway);
		await waitFor(() => expect(gateway.onEvent).toHaveBeenCalled());

		act(() =>
			gateway.emit({ event: 'show.requested', data: { link: VALID_LINK } })
		);

		expect(await screen.findByRole('alertdialog')).toBeInTheDocument();
	});

	it('should ignore a show request without a link', async () => {
		const gateway = createFakeGateway();
		renderHost(gateway);
		await waitFor(() => expect(gateway.onEvent).toHaveBeenCalled());

		act(() => gateway.emit({ event: 'show.requested', data: {} }));

		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
	});

	it('should ignore a second link with a toast while a dialog is shown', async () => {
		const gateway = createFakeGateway();
		renderHost(gateway);
		await waitFor(() => expect(gateway.onEvent).toHaveBeenCalled());
		act(() =>
			gateway.emit({ event: 'show.requested', data: { link: VALID_LINK } })
		);
		await screen.findByRole('alertdialog');

		act(() =>
			gateway.emit({
				event: 'show.requested',
				data: { link: { ...VALID_LINK, instance: 'other-1' } },
			})
		);

		expect(getShownToasts()).toEqual([
			expect.objectContaining({
				message: 'A Tunneler link is already waiting for your answer',
			}),
		]);
		expect(screen.getByRole('alertdialog')).toHaveTextContent('web-shop-098a');
	});
});
