import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { screen, waitFor, within } from '@testing-library/react';

import { Sidebar } from '#/sidebar/sidebar';
import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useDeleteTag } from '#/tag_delete/use_delete_tag';
import { useToastStore } from '#/shared/toasts/toast_store';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';

function Trigger() {
	const deleteTag = useDeleteTag();
	return <button onClick={() => void deleteTag('dev')}>Déclencheur</button>;
}

function renderHook(
	gateway = createFakeGateway(),
	startOnLaunchTags: string[] = []
) {
	useDaemonStore.setState({
		settings: buildSettings({ startOnLaunchTags }),
		tunnels: [
			buildTunnel({ id: 'a', tags: ['dev'] }),
			buildTunnel({ id: 'b', tags: ['dev', 'prod'] }),
			buildTunnel({ id: 'c', tags: ['prod'] }),
		],
	});
	return renderWithGateway(
		<>
			<ModalProvider />
			<Sidebar />
			<Trigger />
		</>,
		gateway
	);
}

async function openConfirmation(user: ReturnType<typeof userEvent.setup>) {
	await user.click(screen.getByRole('button', { name: 'Déclencheur' }));
	return screen.findByRole('alertdialog', { name: 'Supprimer le tag dev ?' });
}

describe('useDeleteTag', () => {
	it('should ask for confirmation before calling the daemon', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHook();

		const dialog = await openConfirmation(user);

		expect(dialog).toHaveTextContent(
			'Le tag sera retiré de 2 tunnels. Les tunnels sont conservés.'
		);
		expect(gateway.deleteTag).not.toHaveBeenCalled();
	});

	it('should mention the launch tags only when the tag is one of them', async () => {
		const user = userEvent.setup();
		renderHook(createFakeGateway(), ['dev']);

		const dialog = await openConfirmation(user);

		expect(dialog).toHaveTextContent(
			'Il sera aussi retiré des « Tags démarrés au lancement ».'
		);
	});

	it('should not mention the launch tags when the tag is not one of them', async () => {
		const user = userEvent.setup();
		renderHook(createFakeGateway(), ['prod']);

		const dialog = await openConfirmation(user);

		expect(dialog).not.toHaveTextContent('Tags démarrés au lancement');
	});

	it('should have no accessibility violation while asking', async () => {
		const user = userEvent.setup();
		renderHook();

		const dialog = await openConfirmation(user);

		expect(await axe(dialog)).toHaveNoViolations();
	});

	it('should call nothing and keep the focus on the trigger when cancelled', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHook();
		const dialog = await openConfirmation(user);

		await user.click(within(dialog).getByRole('button', { name: 'Annuler' }));

		await waitFor(() =>
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		);
		expect(gateway.deleteTag).not.toHaveBeenCalled();
		expect(screen.getByRole('button', { name: 'Déclencheur' })).toHaveFocus();
	});

	it('should delete the tag once when confirmed', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHook();
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer le tag' })
		);

		await waitFor(() => expect(gateway.deleteTag).toHaveBeenCalledTimes(1));
		expect(gateway.deleteTag).toHaveBeenCalledWith('dev');
	});

	it('should toast how many tunnels lost the tag', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.deleteTag.mockResolvedValue({ removedCount: 3 });
		renderHook(gateway);
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer le tag' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'success',
					message: 'Tag dev retiré de 3 tunnels',
				}),
			])
		);
	});

	it('should reset the filter to all tunnels when it was the deleted tag', async () => {
		const user = userEvent.setup();
		renderHook();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer le tag' })
		);

		await waitFor(() =>
			expect(useUiStore.getState().filter).toEqual({ kind: 'all' })
		);
	});

	it('should keep another filter untouched', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHook();
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'prod' });
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer le tag' })
		);

		await waitFor(() => expect(gateway.deleteTag).toHaveBeenCalled());
		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'prod' });
	});

	it('should move the focus to the all tunnels filter', async () => {
		const user = userEvent.setup();
		renderHook();
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer le tag' })
		);

		await waitFor(() =>
			expect(
				screen.getByRole('button', { name: /Tous les tunnels/ })
			).toHaveFocus()
		);
	});

	it('should report a daemon failure in an error toast without moving the filter', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.deleteTag.mockRejectedValue(
			new DaemonCommandError('not_found', 'Aucun tunnel ne porte ce tag')
		);
		renderHook(gateway);
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer le tag' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'error',
					message:
						'Suppression du tag impossible: Aucun tunnel ne porte ce tag',
				}),
			])
		);
		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'dev' });
	});
});
