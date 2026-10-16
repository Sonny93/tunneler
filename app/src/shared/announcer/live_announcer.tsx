import { useAnnouncerStore } from '#/shared/announcer/announcer_store';

/** Both regions are always mounted: a live region must exist before its content changes. */
export function LiveAnnouncer() {
	const polite = useAnnouncerStore((state) => state.polite);
	const assertive = useAnnouncerStore((state) => state.assertive);

	return (
		<>
			<output className="sr-only">
				<span key={polite.id}>{polite.message}</span>
			</output>
			<div role="alert" className="sr-only">
				<span key={assertive.id}>{assertive.message}</span>
			</div>
		</>
	);
}
