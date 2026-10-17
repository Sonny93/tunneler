import { describe, expect, it } from 'vitest';

import type { PaletteCommand } from '#/palette/palette_command';
import {
	filterPaletteCommands,
	MAXIMUM_PALETTE_RESULTS,
} from '#/palette/filter_palette_commands';

const buildCommand = (
	label: string,
	category: PaletteCommand['category'] = 'Tunnel'
): PaletteCommand => ({
	id: label,
	label,
	category,
	action: { kind: 'newTunnel' },
});

const COMMANDS = [
	buildCommand('Démarrer Adonis'),
	buildCommand('Arrêter Adonis'),
	buildCommand('Configuration de nova', 'Hôte'),
	buildCommand('Ouvrir les paramètres', 'Général'),
];

const labelsOf = (commands: readonly PaletteCommand[]) =>
	commands.map((command) => command.label);

describe('filterPaletteCommands', () => {
	it('should keep every command for a blank query', () => {
		expect(filterPaletteCommands(COMMANDS, '   ')).toEqual(COMMANDS);
	});

	it('should ignore case', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'ADONIS'))).toEqual([
			'Démarrer Adonis',
			'Arrêter Adonis',
		]);
	});

	it('should ignore accents on both sides', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'demarrer'))).toEqual([
			'Démarrer Adonis',
		]);
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'ARRÊTER'))).toEqual([
			'Arrêter Adonis',
		]);
	});

	it('should require every word', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'adonis arret'))).toEqual([
			'Arrêter Adonis',
		]);
	});

	it('should match the category too', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'hote'))).toEqual([
			'Configuration de nova',
		]);
	});

	it('should cap the number of results', () => {
		const manyCommands = Array.from(
			{ length: MAXIMUM_PALETTE_RESULTS + 10 },
			(_, index) => buildCommand(`Commande ${index}`)
		);

		expect(filterPaletteCommands(manyCommands, '')).toHaveLength(
			MAXIMUM_PALETTE_RESULTS
		);
	});
});
