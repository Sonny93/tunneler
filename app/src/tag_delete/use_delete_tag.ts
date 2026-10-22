import { t } from '@lingui/core/macro';
import { ConfirmModal } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonAction } from '#/daemon/use_daemon_action';
import {
	describeTagDeletion,
	describeTagDeletionWarning,
} from '#/tag_delete/describe_tag_deletion';

/**
 * Asks for confirmation, then removes the tag from every tunnel. On success it
 * toasts, resets the filter when it was this tag and moves the focus to the
 * "All tunnels" filter, since the tag row and the list header button vanish.
 */
export function useDeleteTag(): (tag: string) => Promise<void> {
	const { i18n } = useLingui();
	const runDaemonAction = useDaemonAction();

	return async (tag) => {
		const { tunnels, settings } = useDaemonStore.getState();
		const tunnelCount = tunnels.filter(({ tags }) => tags.includes(tag)).length;
		const isLaunchTag = settings?.startOnLaunchTags.includes(tag) ?? false;
		const isConfirmed = await ConfirmModal.call({
			title: t(i18n)`Delete tag ${tag}?`,
			children: describeTagDeletionWarning(tunnelCount, isLaunchTag, i18n),
			confirmLabel: t(i18n)`Delete the tag`,
			confirmColor: 'danger',
		});
		if (!isConfirmed) return;
		await runDaemonAction(
			async (gateway) => {
				const { removedCount } = await gateway.deleteTag(tag);
				const ui = useUiStore.getState();
				useToastStore.getState().pushToast({
					tone: 'success',
					message: describeTagDeletion(tag, removedCount, i18n),
				});
				if (ui.filter.kind === 'tag' && ui.filter.tag === tag) {
					ui.setFilter({ kind: 'all' });
				}
				ui.focusAllFilter();
			},
			t(i18n)`Couldn't delete the tag`
		);
	};
}
