import { useLingui } from '@lingui/react/macro';

export function ConnectingScreen() {
	const { t } = useLingui();

	return (
		<main className="p-4">
			<h1 className="text-lg font-semibold">Tunneler</h1>
			<output className="block">{t`Connecting to the daemon…`}</output>
		</main>
	);
}
