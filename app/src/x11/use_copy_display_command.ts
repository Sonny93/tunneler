import { copyTextWithToast } from '#/shared/copy_text';

type UseCopyDisplayCommandReturn = {
	command: string;
	copy: () => Promise<void>;
};

export function useCopyDisplayCommand(
	display: string
): UseCopyDisplayCommandReturn {
	const command = `export DISPLAY=${display}`;

	const copy = () => copyTextWithToast(command, 'Commande copiée');

	return { command, copy };
}
