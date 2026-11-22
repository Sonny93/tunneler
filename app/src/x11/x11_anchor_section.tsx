import { useLingui } from '@lingui/react/macro';
import { Button, Card } from '@minimalstuff/ui';

import { useX11AnchorActions } from '#/x11/use_x11_anchor_actions';
import { useX11State } from '#/x11/use_x11_state';
import { X11LocalDisplay } from '#/x11/x11_local_display';
import { X11StateBadge } from '#/x11/x11_state_badge';
import { getX11StateDetail } from '#/x11/x11_state_label';

type X11AnchorSectionProps = {
	host: string;
};

export function X11AnchorSection({ host }: Readonly<X11AnchorSectionProps>) {
	const { t } = useLingui();
	const state = useX11State(host);
	const { start, stop } = useX11AnchorActions(host);
	const detail = getX11StateDetail(state);
	const isStartable = state.kind === 'stopped' || state.kind === 'failed';

	const handleStart = () => void start();
	const handleStop = () => void stop();

	return (
		<Card title={t`X11 anchor`} headingLevel={3} titleSize="sm" padding="sm">
			<div className="flex flex-col gap-2">
				<div className="flex flex-wrap items-center gap-2">
					<X11StateBadge state={state} />
					{state.kind === 'failed' && detail !== null && (
						<span className="text-sm text-tn-danger">{detail}</span>
					)}
				</div>
				<X11LocalDisplay host={host} />
				<div>
					{isStartable ? (
						<Button startIcon="i-mdi-play" onClick={handleStart}>
							{t`Start X11`}
						</Button>
					) : (
						<Button
							startIcon="i-mdi-stop"
							variant="outline"
							color="neutral"
							onClick={handleStop}
						>
							{t`Stop X11`}
						</Button>
					)}
				</div>
			</div>
		</Card>
	);
}
