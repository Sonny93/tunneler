import type { ReactNode } from 'react';

type SettingsTabContentProps = {
	children: ReactNode;
};

/** The vertical rhythm shared by the panels of every settings tab. */
export const SettingsTabContent = ({
	children,
}: Readonly<SettingsTabContentProps>) => (
	<div className="flex flex-col gap-6">{children}</div>
);
