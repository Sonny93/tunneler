import { ModalProvider } from '@minimalstuff/ui';

import { AppLayout } from '#/app_layout';
import { ConnectingScreen } from '#/connecting_screen';
import { DisconnectedScreen } from '#/disconnected_screen';
import { ExternalSection } from '#/externals/external_section';
import { HeaderActions } from '#/header/header_actions';
import { Overlays } from '#/overlays';
import { LiveAnnouncer } from '#/shared/announcer/live_announcer';
import { ToastRegion } from '#/shared/toasts/toast_region';
import { Sidebar } from '#/sidebar/sidebar';
import { SidebarToggle } from '#/sidebar/sidebar_toggle';
import { useUiStore } from '#/state/ui_store';
import { useDaemonConnection } from '#/state/use_daemon_connection';
import { TrayStatusNotice } from '#/tray/tray_status_notice';
import { TunnelList } from '#/tunnels/list/tunnel_list';
import { useTunnelStateAnnouncements } from '#/tunnels/use_tunnel_state_announcements';
import { UpdateNotice } from '#/updates/update_notice';
import { useUpdateCheck } from '#/updates/use_update_check';
import { useX11Announcements } from '#/x11/use_x11_announcements';

export function App() {
	const { connection, retry } = useDaemonConnection();
	const isSidebarCollapsed = useUiStore((state) => state.isSidebarCollapsed);
	useTunnelStateAnnouncements();
	useX11Announcements();
	useUpdateCheck();

	return (
		<>
			{connection.status === 'connecting' && <ConnectingScreen />}
			{connection.status === 'disconnected' && (
				<DisconnectedScreen error={connection.error} onRetry={retry} />
			)}
			{connection.status === 'connected' && (
				<>
					<AppLayout
						sidebar={<Sidebar />}
						sidebarToggle={<SidebarToggle />}
						isSidebarCollapsed={isSidebarCollapsed}
						actions={<HeaderActions />}
					>
						<TrayStatusNotice />
						<UpdateNotice />
						<TunnelList />
						<ExternalSection />
					</AppLayout>
					<Overlays />
				</>
			)}
			<LiveAnnouncer />
			<ToastRegion />
			<ModalProvider />
		</>
	);
}
