import { useLingui } from '@lingui/react/macro';
import { Button, Card } from '@minimalstuff/ui';

import { useCopyDisplayCommand } from '#/x11/use_copy_display_command';

type X11DisplaySectionProps = {
	display: string;
};

export function X11DisplaySection({
	display,
}: Readonly<X11DisplaySectionProps>) {
	const { t } = useLingui();
	const { command, copy } = useCopyDisplayCommand(display);

	const handleCopy = () => void copy();

	return (
		<Card title={t`Remote display`} headingLevel={3} padding="sm">
			<div className="flex flex-col gap-2">
				<p className="m-0 font-mono select-all">{display}</p>
				<div>
					<Button
						startIcon="i-mdi-content-copy"
						variant="outline"
						color="neutral"
						onClick={handleCopy}
					>
						{t`Copy ${command}`}
					</Button>
				</div>
			</div>
		</Card>
	);
}
