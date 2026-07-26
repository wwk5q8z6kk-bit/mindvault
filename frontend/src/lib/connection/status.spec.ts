// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import {
	connectionStatusLabel,
	deriveConnectionStatus,
	shouldShowConnectionBanner
} from './status';

describe('deriveConnectionStatus', () => {
	it('is offline when the browser is offline', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: false,
				apiStatus: 'healthy',
				wsStatus: 'connected'
			})
		).toBe('offline');
	});

	it('never shows Live when API is offline even if websocket is connected', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: true,
				apiStatus: 'offline',
				wsStatus: 'connected'
			})
		).toBe('offline');
	});

	it('surfaces Degraded when API returns server errors', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: true,
				apiStatus: 'degraded',
				wsStatus: 'connected'
			})
		).toBe('degraded');
	});

	it('shows Connecting until API health is known', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: true,
				apiStatus: 'unknown',
				wsStatus: 'connected'
			})
		).toBe('connecting');
	});

	it('shows Live only when API is healthy and websocket is connected', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: true,
				apiStatus: 'healthy',
				wsStatus: 'connected'
			})
		).toBe('live');
	});

	it('shows Online when API is healthy but websocket is down', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: true,
				apiStatus: 'healthy',
				wsStatus: 'disconnected'
			})
		).toBe('online');
	});

	it('shows Connecting while websocket is connecting and API is healthy', () => {
		expect(
			deriveConnectionStatus({
				browserOnline: true,
				apiStatus: 'healthy',
				wsStatus: 'connecting'
			})
		).toBe('connecting');
	});
});

describe('connectionStatusLabel / banner', () => {
	it('labels statuses for the pill', () => {
		expect(connectionStatusLabel('live')).toBe('Live');
		expect(connectionStatusLabel('degraded')).toBe('Degraded');
		expect(connectionStatusLabel('offline')).toBe('Offline');
	});

	it('only banners offline and degraded', () => {
		expect(shouldShowConnectionBanner('offline')).toBe(true);
		expect(shouldShowConnectionBanner('degraded')).toBe(true);
		expect(shouldShowConnectionBanner('live')).toBe(false);
		expect(shouldShowConnectionBanner('online')).toBe(false);
		expect(shouldShowConnectionBanner('connecting')).toBe(false);
	});
});
