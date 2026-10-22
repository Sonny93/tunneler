const SINGLE_CONNECTION_COUNT = 1;

/** « 1 connexion ouverte sera coupée. », « 2 connexions ouvertes seront coupées. » */
export function describeCutConnections(count: number): string {
	if (count === SINGLE_CONNECTION_COUNT) {
		return '1 connexion ouverte sera coupée.';
	}
	return `${count} connexions ouvertes seront coupées.`;
}
