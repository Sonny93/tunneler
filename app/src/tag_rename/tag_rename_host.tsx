import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';
import { useModalCall } from '#/shared/use_modal_call';
import { TagRenameContent } from '#/tag_rename/tag_rename_content';

const TAG_RENAME_MODAL_ID = 'tag-rename';

/** Opens the rename dialog while `tagRename` is set; a `<ModalProvider />` must be mounted. */
export function TagRenameHost(): null {
	const { t } = useLingui();
	const tagRename = useUiStore((state) => state.tagRename);
	const closeTagRename = useUiStore((state) => state.closeTagRename);

	const tag = tagRename?.tag ?? '';
	const request =
		tagRename === null
			? null
			: {
					id: `${TAG_RENAME_MODAL_ID}:${tag}`,
					props: {
						title: t`Rename tag ${tag}`,
						size: 'sm' as const,
						children: (
							<TagRenameContent
								tag={tag}
								isFocusingRow={tagRename.origin !== 'list'}
							/>
						),
					},
				};
	useModalCall(request, closeTagRename);

	return null;
}
