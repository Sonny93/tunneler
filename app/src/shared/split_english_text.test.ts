import { describe, expect, it } from 'vitest';

import { splitEnglishText } from '#/shared/split_english_text';

describe('splitEnglishText', () => {
	it('should return the whole text as French when there is no fragment', () => {
		expect(splitEnglishText('Le fichier est illisible', [])).toEqual([
			{ text: 'Le fichier est illisible', isEnglish: false },
		]);
	});

	it('should mark the fragment found inside the sentence', () => {
		expect(
			splitEnglishText('Le fichier est illisible (ligne 3) : bad key', [
				'bad key',
			])
		).toEqual([
			{ text: 'Le fichier est illisible (ligne 3) : ', isEnglish: false },
			{ text: 'bad key', isEnglish: true },
		]);
	});

	it('should mark several fragments in order and keep the text between them', () => {
		expect(splitEnglishText('a: one, b: two', ['two', 'one'])).toEqual([
			{ text: 'a: ', isEnglish: false },
			{ text: 'one', isEnglish: true },
			{ text: ', b: ', isEnglish: false },
			{ text: 'two', isEnglish: true },
		]);
	});

	it('should ignore an empty fragment and one that is absent', () => {
		expect(splitEnglishText('texte', ['', 'absent'])).toEqual([
			{ text: 'texte', isEnglish: false },
		]);
	});
});
