const SINGULAR_COUNT = 1;

function pluralizeTunnels(count: number): string {
	return count === SINGULAR_COUNT ? `${count} tunnel` : `${count} tunnels`;
}

export function describeDetectedCount(count: number): string {
	if (count === 0) return 'Aucun tunnel détecté';
	return `${pluralizeTunnels(count)} ${count === SINGULAR_COUNT ? 'détecté' : 'détectés'}`;
}

export function describeImportAction(count: number): string {
	return `Importer ${pluralizeTunnels(count)}`;
}

export function describeImportedCount(count: number): string {
	return `${pluralizeTunnels(count)} ${count === SINGULAR_COUNT ? 'importé' : 'importés'}`;
}
