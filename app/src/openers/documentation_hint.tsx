import { useLingui } from '@lingui/react/macro';

import { useOpenDocumentation } from '#/documentation/use_open_documentation';

/** Points to the guide for applications that are not detected, e.g. a portable version. */
export function DocumentationHint() {
	const { t } = useLingui();
	const openDocumentation = useOpenDocumentation();
	const handleOpenGuide = () => void openDocumentation('applications');

	return (
		<p className="m-0 flex flex-wrap items-center gap-x-2 text-xs text-tn-muted">
			<span>
				{t`Installed somewhere else, like a portable version? Add it below with the full path of its program.`}
			</span>
			<button
				type="button"
				onClick={handleOpenGuide}
				className="m-0 inline-flex min-h-6 cursor-pointer items-center border-0 bg-transparent p-0 text-xs text-tn-text underline"
			>
				{t`How to add an application`}
			</button>
		</p>
	);
}
