/**
 * NoSessionState Component Tests
 */

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { NoSessionState } from '../../src/components/NoSessionState';

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () => ({
  getItem: jest.fn(() => Promise.resolve(null)),
  setItem: jest.fn(() => Promise.resolve()),
  removeItem: jest.fn(() => Promise.resolve()),
  multiRemove: jest.fn(() => Promise.resolve()),
}));

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

describe('NoSessionState', () => {
  describe('when disconnected', () => {
    it('should render "Not Connected" title', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="disconnected" />
      );

      expect(getByText('Not Connected')).toBeTruthy();
    });

    it('should display helpful description for connecting', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="disconnected" />
      );

      expect(
        getByText(
          'Connect to your RustRide desktop app to control workouts and view live metrics.'
        )
      ).toBeTruthy();
    });

    it('should show connect button when onConnectPress is provided', () => {
      const mockOnConnect = jest.fn();
      const { getByText } = render(
        <NoSessionState
          connectionStatus="disconnected"
          onConnectPress={mockOnConnect}
        />
      );

      const connectButton = getByText('Connect to Desktop');
      expect(connectButton).toBeTruthy();
    });

    it('should call onConnectPress when connect button is pressed', () => {
      const mockOnConnect = jest.fn();
      const { getByText } = render(
        <NoSessionState
          connectionStatus="disconnected"
          onConnectPress={mockOnConnect}
        />
      );

      fireEvent.press(getByText('Connect to Desktop'));
      expect(mockOnConnect).toHaveBeenCalledTimes(1);
    });

    it('should not show connect button when onConnectPress is not provided', () => {
      const { queryByText } = render(
        <NoSessionState connectionStatus="disconnected" />
      );

      expect(queryByText('Connect to Desktop')).toBeNull();
    });
  });

  describe('when connecting', () => {
    it('should render "Connecting..." title', () => {
      const { getAllByText } = render(
        <NoSessionState connectionStatus="connecting" />
      );

      // There may be multiple "Connecting..." texts (title and status badge)
      const connectingTexts = getAllByText('Connecting...');
      expect(connectingTexts.length).toBeGreaterThanOrEqual(1);
    });

    it('should display connecting description', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="connecting" />
      );

      expect(
        getByText('Establishing connection to RustRide desktop app.')
      ).toBeTruthy();
    });

    it('should not show connect button', () => {
      const mockOnConnect = jest.fn();
      const { queryByText } = render(
        <NoSessionState
          connectionStatus="connecting"
          onConnectPress={mockOnConnect}
        />
      );

      expect(queryByText('Connect to Desktop')).toBeNull();
    });
  });

  describe('when connected but no session', () => {
    it('should render "Ready to Ride" title', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="connected" />
      );

      expect(getByText('Ready to Ride')).toBeTruthy();
    });

    it('should also show "Ready to Ride" when authenticated', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="authenticated" />
      );

      expect(getByText('Ready to Ride')).toBeTruthy();
    });

    it('should display helpful description about starting session', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="connected" />
      );

      expect(
        getByText(
          'Start a workout or free ride on the desktop app to see live metrics here.'
        )
      ).toBeTruthy();
    });

    it('should display connection status with server name', () => {
      const { getByText } = render(
        <NoSessionState
          connectionStatus="authenticated"
          serverName="MyPC:9876"
        />
      );

      expect(getByText('MyPC:9876')).toBeTruthy();
    });

    it('should display tips when connected', () => {
      const { getByText } = render(
        <NoSessionState connectionStatus="connected" />
      );

      expect(
        getByText('Use your phone as a remote control during workouts')
      ).toBeTruthy();
      expect(
        getByText('View real-time power, heart rate, and cadence')
      ).toBeTruthy();
    });

    it('should not show connect button when connected', () => {
      const mockOnConnect = jest.fn();
      const { queryByText } = render(
        <NoSessionState
          connectionStatus="connected"
          onConnectPress={mockOnConnect}
        />
      );

      expect(queryByText('Connect to Desktop')).toBeNull();
    });
  });

  describe('accessibility', () => {
    it('should have proper accessibility label when disconnected', () => {
      const { getByLabelText } = render(
        <NoSessionState connectionStatus="disconnected" />
      );

      expect(
        getByLabelText(
          'Not Connected. Connect to your RustRide desktop app to control workouts and view live metrics.'
        )
      ).toBeTruthy();
    });

    it('should have proper accessibility label when connected', () => {
      const { getByLabelText } = render(
        <NoSessionState connectionStatus="connected" />
      );

      expect(
        getByLabelText(
          'Ready to Ride. Start a workout or free ride on the desktop app to see live metrics here.'
        )
      ).toBeTruthy();
    });

    it('should have proper accessibility label when connecting', () => {
      const { getByLabelText } = render(
        <NoSessionState connectionStatus="connecting" />
      );

      expect(
        getByLabelText(
          'Connecting.... Establishing connection to RustRide desktop app.'
        )
      ).toBeTruthy();
    });
  });

  describe('styling', () => {
    it('should apply custom style', () => {
      const { toJSON } = render(
        <NoSessionState
          connectionStatus="disconnected"
          style={{ marginTop: 20 }}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });

    it('should render with server name', () => {
      const { toJSON } = render(
        <NoSessionState
          connectionStatus="authenticated"
          serverName="Desktop PC:9876"
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });
  });
});
