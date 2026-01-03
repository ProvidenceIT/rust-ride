/**
 * SettingsScreen Tests
 *
 * Tests for the settings screen with sections for Connection, Display, and Feedback.
 */

import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { SettingsScreen } from '../../src/screens/SettingsScreen';
import { ThemeProvider } from '../../src/theme';
import { useSettingsStore } from '../../src/stores/settingsStore';
import { useConnectionStore } from '../../src/stores/connectionStore';

// Mock navigation - must mock useNavigation from @react-navigation/native
const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => {
  const actual = jest.requireActual('@react-navigation/native');
  return {
    ...actual,
    useNavigation: () => ({
      navigate: mockNavigate,
      goBack: jest.fn(),
    }),
  };
});

/* eslint-disable @typescript-eslint/no-explicit-any */
const mockNavigation: any = {
  navigate: mockNavigate,
  goBack: jest.fn(),
  setOptions: jest.fn(),
  getState: jest.fn(),
  reset: jest.fn(),
  dispatch: jest.fn(),
  canGoBack: jest.fn(),
  getId: jest.fn(),
  getParent: jest.fn(),
  setParams: jest.fn(),
  addListener: jest.fn(),
  removeListener: jest.fn(),
  isFocused: jest.fn(),
};

const mockRoute: any = {
  key: 'Settings',
  name: 'Settings',
  params: undefined,
};
/* eslint-enable @typescript-eslint/no-explicit-any */

// Helper to render with theme and navigation
const renderWithProviders = () => {
  return render(
    <ThemeProvider>
      <SettingsScreen
        navigation={mockNavigation}
        route={mockRoute}
      />
    </ThemeProvider>
  );
};

// Reset stores before each test
beforeEach(() => {
  useSettingsStore.setState({
    settings: {
      units: 'metric',
      keepScreenAwake: true,
      hapticFeedback: 'medium',
      theme: 'system',
    },
    isLoaded: false,
    isSaving: false,
  });
  useConnectionStore.getState().reset();
  jest.clearAllMocks();
});

describe('SettingsScreen', () => {
  describe('Layout and Structure', () => {
    it('renders the settings title', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('Settings')).toBeTruthy();
    });

    it('renders all sections', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('CONNECTION')).toBeTruthy();
      expect(getByText('DISPLAY')).toBeTruthy();
      expect(getByText('FEEDBACK')).toBeTruthy();
      expect(getByText('ABOUT')).toBeTruthy();
    });

    it('renders all setting rows', () => {
      const { getByTestId } = renderWithProviders();

      expect(getByTestId('setting-server')).toBeTruthy();
      expect(getByTestId('setting-units')).toBeTruthy();
      expect(getByTestId('setting-theme')).toBeTruthy();
      expect(getByTestId('setting-keep-awake')).toBeTruthy();
      expect(getByTestId('setting-haptic')).toBeTruthy();
      expect(getByTestId('setting-version')).toBeTruthy();
    });
  });

  describe('Connection Section', () => {
    it('shows "Not connected" when disconnected', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('Not connected')).toBeTruthy();
    });

    it('shows "Connecting..." when connecting', () => {
      useConnectionStore.setState({ status: 'connecting' });

      const { getByText } = renderWithProviders();

      expect(getByText('Connecting...')).toBeTruthy();
    });

    it('shows server name when authenticated', () => {
      useConnectionStore.setState({
        status: 'authenticated',
        isAuthenticated: true,
        currentServer: { name: 'My PC', host: '192.168.1.100', port: 9876 },
      });

      const { getByText } = renderWithProviders();

      expect(getByText('My PC')).toBeTruthy();
    });

    it('navigates to Connection screen when server row is pressed', () => {
      const { getByTestId } = renderWithProviders();

      fireEvent.press(getByTestId('setting-server'));

      expect(mockNavigate).toHaveBeenCalledWith('Connection');
    });
  });

  describe('Display Section', () => {
    it('shows current units value', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('Metric')).toBeTruthy();
    });

    it('shows imperial when units is imperial', () => {
      useSettingsStore.setState({
        settings: {
          units: 'imperial',
          keepScreenAwake: true,
          hapticFeedback: 'medium',
          theme: 'system',
        },
        isLoaded: true,
        isSaving: false,
      });

      const { getByText } = renderWithProviders();

      expect(getByText('Imperial')).toBeTruthy();
    });

    it('shows current theme value', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('System')).toBeTruthy();
    });

    it('shows keep screen awake toggle in correct state', () => {
      const { getByTestId } = renderWithProviders();

      const toggle = getByTestId('setting-keep-awake');
      // The toggle should be on by default
      expect(toggle.props.value).toBe(true);
    });

    it('opens units picker when units row is pressed', async () => {
      const { getByTestId, getByText } = renderWithProviders();

      fireEvent.press(getByTestId('setting-units'));

      await waitFor(() => {
        expect(getByText('Select Units')).toBeTruthy();
      });
    });

    it('opens theme picker when theme row is pressed', async () => {
      const { getByTestId, getByText } = renderWithProviders();

      fireEvent.press(getByTestId('setting-theme'));

      await waitFor(() => {
        expect(getByText('Select Theme')).toBeTruthy();
      });
    });

    it('toggles keep screen awake when switch is pressed', async () => {
      const { getByTestId } = renderWithProviders();

      const toggle = getByTestId('setting-keep-awake');
      fireEvent(toggle, 'valueChange', false);

      await waitFor(() => {
        const settings = useSettingsStore.getState().settings;
        expect(settings.keepScreenAwake).toBe(false);
      });
    });
  });

  describe('Feedback Section', () => {
    it('shows current haptic feedback value', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('Medium')).toBeTruthy();
    });

    it('opens haptic picker when haptic row is pressed', async () => {
      const { getByTestId } = renderWithProviders();

      fireEvent.press(getByTestId('setting-haptic'));

      // Verify the picker opened by looking for an option
      await waitFor(() => {
        expect(getByTestId('option-off')).toBeTruthy();
      });
    });
  });

  describe('About Section', () => {
    it('shows version number', () => {
      const { getByText } = renderWithProviders();

      expect(getByText('1.0.0')).toBeTruthy();
    });

    it('version row does not have chevron', () => {
      const { getByTestId } = renderWithProviders();

      // Version row should not be pressable (no navigation)
      const versionRow = getByTestId('setting-version');
      expect(versionRow.props.accessibilityHint).toBeUndefined();
    });
  });

  describe('Settings Persistence', () => {
    it('calls loadSettings on mount', () => {
      const loadSettings = jest.spyOn(useSettingsStore.getState(), 'loadSettings');

      renderWithProviders();

      expect(loadSettings).toHaveBeenCalled();
    });

    it('updates units setting when selected', async () => {
      const { getByTestId } = renderWithProviders();

      // Open units picker
      fireEvent.press(getByTestId('setting-units'));

      // Wait for picker to render
      await waitFor(() => {
        expect(getByTestId('option-imperial')).toBeTruthy();
      });

      // Select imperial
      fireEvent.press(getByTestId('option-imperial'));

      await waitFor(() => {
        const settings = useSettingsStore.getState().settings;
        expect(settings.units).toBe('imperial');
      });
    });

    it('updates theme setting when selected', async () => {
      const { getByTestId } = renderWithProviders();

      // Open theme picker
      fireEvent.press(getByTestId('setting-theme'));

      // Wait for picker to render
      await waitFor(() => {
        expect(getByTestId('option-dark')).toBeTruthy();
      });

      // Select dark
      fireEvent.press(getByTestId('option-dark'));

      await waitFor(() => {
        const settings = useSettingsStore.getState().settings;
        expect(settings.theme).toBe('dark');
      });
    });

    it('updates haptic setting when selected', async () => {
      const { getByTestId } = renderWithProviders();

      // Open haptic picker
      fireEvent.press(getByTestId('setting-haptic'));

      // Wait for picker to render
      await waitFor(() => {
        expect(getByTestId('option-strong')).toBeTruthy();
      });

      // Select strong
      fireEvent.press(getByTestId('option-strong'));

      await waitFor(() => {
        const settings = useSettingsStore.getState().settings;
        expect(settings.hapticFeedback).toBe('strong');
      });
    });
  });

  describe('Picker Modal Behavior', () => {
    it('closes units picker after selection', async () => {
      const { getByTestId, queryByText } = renderWithProviders();

      // Open units picker
      fireEvent.press(getByTestId('setting-units'));

      await waitFor(() => {
        expect(getByTestId('option-imperial')).toBeTruthy();
      });

      // Select an option
      fireEvent.press(getByTestId('option-imperial'));

      // Modal should close
      await waitFor(() => {
        expect(queryByText('Select Units')).toBeNull();
      });
    });

    it('closes picker when close button is pressed', async () => {
      const { getByTestId, getByLabelText, queryByText } = renderWithProviders();

      // Open units picker
      fireEvent.press(getByTestId('setting-units'));

      await waitFor(() => {
        expect(getByLabelText('Close')).toBeTruthy();
      });

      // Press close button
      fireEvent.press(getByLabelText('Close'));

      // Modal should close
      await waitFor(() => {
        expect(queryByText('Select Units')).toBeNull();
      });
    });
  });

  describe('Accessibility', () => {
    it('setting rows have accessible labels', () => {
      const { getByTestId } = renderWithProviders();

      expect(getByTestId('setting-server').props.accessibilityLabel).toBe('Server');
      expect(getByTestId('setting-units').props.accessibilityLabel).toBe('Units');
      expect(getByTestId('setting-theme').props.accessibilityLabel).toBe('Theme');
    });

    it('toggle row has accessible properties', () => {
      const { getByLabelText } = renderWithProviders();

      // Find the toggle by its accessibility label
      const keepAwakeToggle = getByLabelText('Keep Screen Awake');
      expect(keepAwakeToggle).toBeTruthy();
    });

    it('setting rows with chevron have accessibility hints', () => {
      const { getByTestId } = renderWithProviders();

      expect(getByTestId('setting-server').props.accessibilityHint).toContain('Opens settings');
      expect(getByTestId('setting-units').props.accessibilityHint).toContain('Opens settings');
    });
  });
});
