import type { ReactNode } from 'react';

import { DrawerPanel } from '#/shared/drawer_panel';

type DrawerProps = {
	isOpen: boolean;
	title: string;
	onClose: () => void;
	children: ReactNode;
	footer?: ReactNode;
};

export const Drawer = ({ isOpen, ...panelProps }: Readonly<DrawerProps>) =>
	isOpen ? <DrawerPanel {...panelProps} /> : null;
