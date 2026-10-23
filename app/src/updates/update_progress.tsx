import { useLingui } from '@lingui/react/macro';
import { useEffect, useRef } from 'react';

import { describeUpdateProgress } from '#/updates/describe_update_progress';
import { useUpdateStore } from '#/updates/update_store';

/**
 * The progress of the installation, as a labelled progress bar and the same
 * text. It takes the focus when it appears, because the button that started
 * the installation is gone.
 */
export function UpdateProgress() {
	const { t, i18n } = useLingui();
	const progress = useUpdateStore((state) => state.progress);
	const { percent, text } = describeUpdateProgress(progress, i18n);
	const containerRef = useRef<HTMLDivElement>(null);

	useEffect(() => containerRef.current?.focus(), []);

	return (
		<div
			ref={containerRef}
			tabIndex={-1}
			className="flex w-full max-w-md flex-col gap-1"
		>
			<progress
				aria-label={t`Update progress`}
				aria-valuetext={text}
				max={100}
				value={percent ?? undefined}
				className="w-full"
			/>
			<p className="m-0 text-sm">{text}</p>
		</div>
	);
}
