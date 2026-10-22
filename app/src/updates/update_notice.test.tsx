import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor } from '@testing-library/react';

import { useDaemonStore } from '#/state/daemon_store';
import { UpdateNotice } from '#/updates/update_notice';
import { useUpdateStore } from '#/updates/update_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { UpdateOfferDto, UpdateProgressDto } from '#/daemon/protocol';

const RELEASE_URL = 'https://github.com/Sonny93/tunneler/releases/tag/v0.2.0';

const buildOffer = (
	overrides: Readonly<Partial<UpdateOfferDto>> = {}
): UpdateOfferDto => ({
	version: '0.2.0',
	releaseUrl: RELEASE_URL,
	action: 'installAndRestart',
	...overrides,
});

const renderNotice = (
	offer: UpdateOfferDto | null = buildOffer(),
	gateway = createFakeGateway()
) => {
	useUpdateStore.setState({ offer });
	return renderWithGateway(
		<main tabIndex={-1}>
			<ModalProvider />
			<UpdateNotice />
		</main>,
		gateway
	);
};

const getRegion = () =>
	screen.getByRole('region', { name: 'Version 0.2.0 available' });

describe('UpdateNotice', () => {
	it('should render nothing when no newer version was found', () => {
		renderNotice(null);

		expect(screen.queryByRole('region')).not.toBeInTheDocument();
	});

	it('should announce the new version in a titled region', () => {
		renderNotice();

		expect(getRegion()).toBeInTheDocument();
	});

	it('should show the release notes when the release has some', () => {
		renderNotice(buildOffer({ notes: 'Fixes reconnection' }));

		expect(getRegion()).toHaveTextContent('Fixes reconnection');
	});

	it('should offer to install and restart when the program can update itself', () => {
		renderNotice(buildOffer({ action: 'installAndRestart' }));

		expect(
			screen.getByRole('button', { name: 'Install and restart' })
		).toBeInTheDocument();
	});

	it('should only link the release when the program cannot update itself', () => {
		renderNotice(buildOffer({ action: 'releaseLinkOnly' }));

		expect(
			screen.queryByRole('button', { name: 'Install and restart' })
		).not.toBeInTheDocument();
		expect(
			screen.getByRole('link', { name: 'View release' })
		).toBeInTheDocument();
	});

	it('should open the release page through the native side when the link is clicked', async () => {
		const { gateway } = renderNotice();

		await userEvent.click(screen.getByRole('link', { name: 'View release' }));

		expect(gateway.openReleasePage).toHaveBeenCalledOnce();
	});

	it('should toast an error when the release page cannot open', async () => {
		const gateway = createFakeGateway();
		gateway.openReleasePage.mockRejectedValue(
			new DaemonCommandError('no_browser', 'no browser')
		);
		renderNotice(buildOffer(), gateway);

		await userEvent.click(screen.getByRole('link', { name: 'View release' }));

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				"Couldn't open the page: no browser"
			)
		);
	});

	it('should hide the notice and move the focus to the main region on Hide', async () => {
		renderNotice();

		await userEvent.click(screen.getByRole('button', { name: 'Hide' }));

		expect(screen.queryByRole('region')).not.toBeInTheDocument();
		expect(screen.getByRole('main')).toHaveFocus();
	});

	it('should install without confirmation when no tunnel runs', async () => {
		const { gateway } = renderNotice();

		await userEvent.click(
			screen.getByRole('button', { name: 'Install and restart' })
		);

		expect(gateway.installUpdate).toHaveBeenCalledOnce();
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
	});

	describe('when tunnels run', () => {
		const runWithTunnels = (restoreOnLaunch: boolean) =>
			useDaemonStore.setState({
				tunnels: [buildTunnel({ state: { kind: 'active' } })],
				settings: buildSettings({ restoreOnLaunch }),
			});

		it('should ask for confirmation, saying the tunnels restart when they are restored at launch', async () => {
			runWithTunnels(true);
			const { gateway } = renderNotice();

			await userEvent.click(
				screen.getByRole('button', { name: 'Install and restart' })
			);

			const dialog = await screen.findByRole('alertdialog', {
				name: 'Install the update?',
			});
			expect(dialog).toHaveTextContent(
				'1 running tunnel or X11 session will be stopped.'
			);
			expect(dialog).toHaveTextContent('They will restart after the update');
			expect(await axe(document.body)).toHaveNoViolations();
			expect(gateway.installUpdate).not.toHaveBeenCalled();
		});

		it('should say the tunnels stay stopped when they are not restored at launch', async () => {
			runWithTunnels(false);
			renderNotice();

			await userEvent.click(
				screen.getByRole('button', { name: 'Install and restart' })
			);

			expect(
				await screen.findByRole('alertdialog', {
					name: 'Install the update?',
				})
			).toHaveTextContent('They will not restart after the update');
		});

		it('should install once the confirmation is accepted', async () => {
			runWithTunnels(true);
			const { gateway } = renderNotice();
			await userEvent.click(
				screen.getByRole('button', { name: 'Install and restart' })
			);

			await userEvent.click(
				await screen.findByRole('button', {
					name: 'Stop everything and install',
				})
			);

			await waitFor(() => expect(gateway.installUpdate).toHaveBeenCalledOnce());
		});

		it('should not install when the confirmation is cancelled', async () => {
			runWithTunnels(true);
			const { gateway } = renderNotice();
			await userEvent.click(
				screen.getByRole('button', { name: 'Install and restart' })
			);

			await userEvent.click(
				await screen.findByRole('button', { name: 'Cancel' })
			);

			await waitFor(() =>
				expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
			);
			expect(gateway.installUpdate).not.toHaveBeenCalled();
		});
	});

	describe('while installing', () => {
		const startInstallation = async () => {
			const gateway = createFakeGateway();
			let reportProgress: (progress: UpdateProgressDto) => void = () =>
				undefined;
			gateway.installUpdate.mockImplementation((onProgress) => {
				reportProgress = onProgress;
				return new Promise(() => undefined);
			});
			renderNotice(buildOffer(), gateway);
			await userEvent.click(
				screen.getByRole('button', { name: 'Install and restart' })
			);
			return {
				gateway,
				report: (progress: UpdateProgressDto) =>
					act(() => reportProgress(progress)),
			};
		};

		it('should replace the buttons by a labelled progress bar that takes the focus', async () => {
			await startInstallation();

			const progressBar = await screen.findByRole('progressbar', {
				name: 'Update progress',
			});
			expect(progressBar).toHaveAttribute(
				'aria-valuetext',
				'Downloading the update…'
			);
			expect(progressBar.parentElement).toHaveFocus();
			expect(
				screen.queryByRole('button', { name: 'Install and restart' })
			).not.toBeInTheDocument();
		});

		it('should tell the percentage and the sizes as text and value text', async () => {
			const { report } = await startInstallation();

			report({ kind: 'downloading', downloadedBytes: 512, totalBytes: 1024 });

			const progressBar = await screen.findByRole('progressbar');
			expect(progressBar).toHaveAttribute(
				'aria-valuetext',
				'Downloading: 50% (512 B of 1 KB)'
			);
			expect(progressBar).toHaveValue(50);
			expect(getRegion()).toHaveTextContent('Downloading: 50%');
		});

		it('should tell the installation runs once the download is over', async () => {
			const { report } = await startInstallation();

			report({ kind: 'installing' });

			expect(await screen.findByRole('progressbar')).toHaveAttribute(
				'aria-valuetext',
				'Installing, Tunneler will restart.'
			);
		});

		it('should have no axe violations', async () => {
			const { report } = await startInstallation();
			report({ kind: 'downloading', downloadedBytes: 512, totalBytes: 1024 });

			expect(await axe(document.body)).toHaveNoViolations();
		});
	});

	it('should toast the failure and offer the buttons again when the installation fails', async () => {
		const gateway = createFakeGateway();
		gateway.installUpdate.mockRejectedValue(
			new DaemonCommandError('update_failed', 'network down')
		);
		renderNotice(buildOffer(), gateway);

		await userEvent.click(
			screen.getByRole('button', { name: 'Install and restart' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]).toMatchObject({
				tone: 'error',
				message: "Couldn't install the update: The update failed",
			})
		);
		expect(
			await screen.findByRole('button', { name: 'Install and restart' })
		).toBeInTheDocument();
	});

	it.each<UpdateOfferDto['action']>(['installAndRestart', 'releaseLinkOnly'])(
		'should have no axe violations for the %s action with notes',
		async (action) => {
			const { container } = renderNotice(
				buildOffer({ action, notes: 'Fixes reconnection' })
			);

			expect(await axe(container)).toHaveNoViolations();
		}
	);
});
