import { useLingui } from '@lingui/react/macro';

import { copyTextWithToast } from '#/shared/copy_text';

type UseCopyDisplayCommandReturn = {
	command: string;
	copy: () => Promise<void>;
};

export function useCopyDisplayCommand(
	display: string
): UseCopyDisplayCommandReturn {
	const { t } = useLingui();
	const command = `export DISPLAY=${display}`;

	const copy = () => copyTextWithToast(command, t`Command copied`);

	return { command, copy };
}
