import { useUiStore } from '#/state/ui_store';
import { TooltipIconButton } from '#/shared/tooltip_icon_button';

type HostConfigButtonProps = {
	host: string;
};

/** Opens the resolved ssh configuration of a host. */
export function HostConfigButton({ host }: Readonly<HostConfigButtonProps>) {
	const openHostConfig = useUiStore((state) => state.openHostConfig);

	const handleOpen = () => openHostConfig(host);

	return (
		<TooltipIconButton
			icon="i-mdi-file-cog-outline"
			aria-label={`Configuration de ${host}`}
			variant="ghost"
			color="neutral"
			size="sm"
			onClick={handleOpen}
		/>
	);
}
