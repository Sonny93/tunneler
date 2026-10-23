import { ModalProvider } from '@minimalstuff/ui';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import { DaemonCommandError } from '#/daemon/daemon_command_error';
import { useToastStore } from '#/shared/toasts/toast_store';
import { Sidebar } from '#/sidebar/sidebar';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';
import { useDeleteTag } from '#/tag_delete/use_delete_tag';
import { buildSettings, buildTunnel } from '#/testing/factories';
import { createFakeGateway } from '#/testing/fake_gateway';
import { renderWithGateway } from '#/testing/render_with_gateway';

function Trigger() {
	const deleteTag = useDeleteTag();
	return <button onClick={() => void deleteTag('dev')}>Trigger</button>;
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
	await user.click(screen.getByRole('button', { name: 'Trigger' }));
	return screen.findByRole('alertdialog', { name: 'Delete tag dev?' });
}

describe('useDeleteTag', () => {
	it('should ask for confirmation before calling the daemon', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHook();

		const dialog = await openConfirmation(user);

		expect(dialog).toHaveTextContent(
			'The tag will be removed from 2 tunnels. The tunnels are kept.'
		);
		expect(gateway.deleteTag).not.toHaveBeenCalled();
	});

	it('should mention the launch tags only when the tag is one of them', async () => {
		const user = userEvent.setup();
		renderHook(createFakeGateway(), ['dev']);

		const dialog = await openConfirmation(user);

		expect(dialog).toHaveTextContent(
			'It will also be removed from the "Tags to start on launch".'
		);
	});

	it('should not mention the launch tags when the tag is not one of them', async () => {
		const user = userEvent.setup();
		renderHook(createFakeGateway(), ['prod']);

		const dialog = await openConfirmation(user);

		expect(dialog).not.toHaveTextContent('Tags to start on launch');
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

		await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));

		await waitFor(() =>
			expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		);
		expect(gateway.deleteTag).not.toHaveBeenCalled();
		expect(screen.getByRole('button', { name: 'Trigger' })).toHaveFocus();
	});

	it('should delete the tag once when confirmed', async () => {
		const user = userEvent.setup();
		const { gateway } = renderHook();
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Delete the tag' })
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
			within(dialog).getByRole('button', { name: 'Delete the tag' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'success',
					message: 'Tag dev removed from 3 tunnels',
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
			within(dialog).getByRole('button', { name: 'Delete the tag' })
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
			within(dialog).getByRole('button', { name: 'Delete the tag' })
		);

		await waitFor(() => expect(gateway.deleteTag).toHaveBeenCalled());
		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'prod' });
	});

	it('should move the focus to the all tunnels filter', async () => {
		const user = userEvent.setup();
		renderHook();
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Delete the tag' })
		);

		await waitFor(() =>
			expect(screen.getByRole('button', { name: /All tunnels/ })).toHaveFocus()
		);
	});

	it('should report a daemon failure in an error toast without moving the filter', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.deleteTag.mockRejectedValue(
			new DaemonCommandError('not_found', 'No tunnel has this tag')
		);
		renderHook(gateway);
		useUiStore.getState().setFilter({ kind: 'tag', tag: 'dev' });
		const dialog = await openConfirmation(user);

		await user.click(
			within(dialog).getByRole('button', { name: 'Delete the tag' })
		);

		await waitFor(() =>
			expect(useToastStore.getState().toasts).toEqual([
				expect.objectContaining({
					tone: 'error',
					message: "Couldn't delete the tag: No tunnel has this tag",
				}),
			])
		);
		expect(useUiStore.getState().filter).toEqual({ kind: 'tag', tag: 'dev' });
	});
});
