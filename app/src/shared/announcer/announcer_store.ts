import { create } from 'zustand';

export type Politeness = 'polite' | 'assertive';

export type Announcement = { id: number; message: string };

type AnnouncerState = {
	polite: Announcement;
	assertive: Announcement;
	announce: (message: string, politeness: Politeness) => void;
};

const EMPTY_ANNOUNCEMENT: Announcement = { id: 0, message: '' };

/**
 * Each announcement gets a new `id` that the live region uses as a React key,
 * so announcing an identical message again remounts its text and is read again.
 */
export const useAnnouncerStore = create<AnnouncerState>()((set) => ({
	polite: EMPTY_ANNOUNCEMENT,
	assertive: EMPTY_ANNOUNCEMENT,
	announce: (message, politeness) =>
		set((state) => ({
			[politeness]: { id: state[politeness].id + 1, message },
		})),
}));

export const INITIAL_ANNOUNCER_STATE = {
	polite: EMPTY_ANNOUNCEMENT,
	assertive: EMPTY_ANNOUNCEMENT,
};
