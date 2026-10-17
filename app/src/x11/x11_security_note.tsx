import { useId } from 'react';

export function X11SecurityNote() {
	const titleId = useId();

	return (
		<div
			role="note"
			aria-labelledby={titleId}
			className="flex flex-col gap-1 rounded-md border border-tn-warning bg-tn-warning-subtle p-3 text-sm text-tn-text"
		>
			<p id={titleId} className="m-0 flex items-center gap-1.5 font-semibold">
				<span aria-hidden="true" className="i-mdi-alert size-4 shrink-0" />
				Sécurité
			</p>
			<p className="m-0">
				Le transfert X11 donne au serveur un accès complet à votre affichage
				local : il peut capturer le clavier et l'écran. À n'activer que vers des
				hôtes de confiance.
			</p>
		</div>
	);
}
