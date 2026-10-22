import { IconButton } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';

type HostConfigButtonProps = {
	host: string;
};

/** Opens the resolved ssh configuration of a host. */
export function HostConfigButton({ host }: Readonly<HostConfigButtonProps>) {
	const { t } = useLingui();
	const openHostConfig = useUiStore((state) => state.openHostConfig);

	const handleOpen = () => openHostConfig(host);

	return (
		<IconButton
			icon="i-mdi-file-cog-outline"
			aria-label={t`Configuration of ${host}`}
			tooltip
			variant="ghost"
			color="neutral"
			size="sm"
			onClick={handleOpen}
		/>
	);
}
