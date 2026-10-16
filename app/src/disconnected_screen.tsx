import { Button } from '@minimalstuff/ui';

type DisconnectedScreenProps = {
	message: string;
	onRetry: () => void;
};

export const DisconnectedScreen = ({
	message,
	onRetry,
}: Readonly<DisconnectedScreenProps>) => (
	<main className="flex flex-col items-start gap-3 p-4">
		<h1 className="text-lg font-semibold">Tunneler</h1>
		<p role="alert">Daemon injoignable : {message}</p>
		<Button onClick={onRetry}>Réessayer</Button>
	</main>
);
