import { Button } from '@minimalstuff/ui';

import { useQuitFlow } from '#/quit/use_quit_flow';

export function QuitButton() {
	const { quit } = useQuitFlow();

	const handleClick = () => void quit();

	return (
		<Button
			variant="ghost"
			color="neutral"
			startIcon="i-mdi-power"
			onClick={handleClick}
		>
			Quitter Tunneler
		</Button>
	);
}
