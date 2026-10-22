import { Button } from '@minimalstuff/ui';
import { useLingui } from '@lingui/react/macro';

import { useX11State } from '#/x11/use_x11_state';
import { X11StateBadge } from '#/x11/x11_state_badge';
import { getX11StateDetail } from '#/x11/x11_state_label';
import { useX11AnchorActions } from '#/x11/use_x11_anchor_actions';

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
		<section className="flex flex-col gap-2">
			<h3 className="m-0 text-sm font-semibold">{t`X11 anchor`}</h3>
			<div className="flex flex-wrap items-center gap-2">
				<X11StateBadge state={state} />
				{state.kind === 'failed' && detail !== null && (
					<span className="text-sm text-tn-danger">{detail}</span>
				)}
			</div>
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
		</section>
	);
}
