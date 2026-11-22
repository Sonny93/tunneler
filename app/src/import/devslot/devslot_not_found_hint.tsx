import { useLingui } from '@lingui/react/macro';
import { Button, CopyButton } from '@minimalstuff/ui';

import { buildDevslotConfigSnippet } from '#/import/devslot/build_devslot_config_snippet';

type DevslotNotFoundHintProps = {
	host: string;
};

/** How to tell Tunneler where `ds` is on a host, with the alias of that host filled in. */
export function DevslotNotFoundHint({
	host,
}: Readonly<DevslotNotFoundHintProps>) {
	const { t } = useLingui();
	const snippet = buildDevslotConfigSnippet(host);

	return (
		<div className="flex flex-col gap-2 text-sm">
			<p className="m-0">
				{t`If ds is installed elsewhere on this host, add its path to the configuration file:`}
			</p>
			<pre
				lang="en"
				className="m-0 rounded-md bg-tn-neutral-subtle p-3 break-all whitespace-pre-wrap"
			>
				<code>{snippet}</code>
			</pre>
			<div>
				<CopyButton value={snippet}>
					{({ copied, copy }) => {
						const handleCopy = () => void copy();
						return (
							<Button
								size="sm"
								variant="outline"
								color="neutral"
								startIcon={copied ? 'i-mdi-check' : 'i-mdi-content-copy'}
								onClick={handleCopy}
							>
								{copied ? t`Copied` : t`Copy configuration snippet`}
							</Button>
						);
					}}
				</CopyButton>
			</div>
		</div>
	);
}
