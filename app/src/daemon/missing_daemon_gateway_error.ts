export class MissingDaemonGatewayError extends Error {
	readonly code = 'E_MISSING_DAEMON_GATEWAY';

	constructor() {
		super('useDaemonGateway must be used inside a DaemonGatewayProvider');
		this.name = 'MissingDaemonGatewayError';
	}
}
