import { create } from 'zustand';

import type { UpdateOfferDto, UpdateProgressDto } from '#/daemon/protocol';

type UpdateState = {
	/** The newer version the check found. */
	offer: UpdateOfferDto | null;
	/** Whether the check already ran during this UI session: it runs once. */
	hasChecked: boolean;
	/** Whether the user hid the banner for this UI session. */
	isHidden: boolean;
	isInstalling: boolean;
	/** The last progress of the installation; `null` before the first one. */
	progress: UpdateProgressDto | null;
};

type UpdateActions = {
	markChecked: () => void;
	setOffer: (offer: UpdateOfferDto | null) => void;
	/** Stores the offer and shows the banner again if the user had hidden it. */
	showOffer: (offer: UpdateOfferDto) => void;
	hide: () => void;
	startInstallation: () => void;
	reportProgress: (progress: UpdateProgressDto) => void;
	endInstallation: () => void;
};

export const INITIAL_UPDATE_STATE: UpdateState = {
	offer: null,
	hasChecked: false,
	isHidden: false,
	isInstalling: false,
	progress: null,
};

export const useUpdateStore = create<UpdateState & UpdateActions>()((set) => ({
	...INITIAL_UPDATE_STATE,
	markChecked: () => set({ hasChecked: true }),
	setOffer: (offer) => set({ offer }),
	showOffer: (offer) => set({ offer, isHidden: false }),
	hide: () => set({ isHidden: true }),
	startInstallation: () => set({ isInstalling: true, progress: null }),
	reportProgress: (progress) => set({ progress }),
	endInstallation: () => set({ isInstalling: false, progress: null }),
}));
