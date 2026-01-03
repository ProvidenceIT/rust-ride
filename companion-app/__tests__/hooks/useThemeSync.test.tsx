/**
 * useThemeSync Hook Tests
 *
 * Tests for the theme synchronization between settingsStore and ThemeProvider.
 */

import React from 'react';
import { Text, View } from 'react-native';
import { render, act, waitFor } from '@testing-library/react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { ThemeProvider, useThemeContext } from '../../src/theme';
import { useThemeSync } from '../../src/hooks/useThemeSync';
import { useSettingsStore } from '../../src/stores/settingsStore';

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () => ({
  getItem: jest.fn(() => Promise.resolve(null)),
  setItem: jest.fn(() => Promise.resolve()),
  removeItem: jest.fn(() => Promise.resolve()),
}));

// Get mocked AsyncStorage for test manipulation
const mockedAsyncStorage = AsyncStorage as jest.Mocked<typeof AsyncStorage>;

// Mock useColorScheme
let mockColorScheme: 'dark' | 'light' | null = 'dark';

jest.mock('react-native/Libraries/Utilities/useColorScheme', () => ({
  default: jest.fn(() => mockColorScheme),
}));

jest.mock('react-native/Libraries/Utilities/Appearance', () => ({
  addChangeListener: jest.fn(() => ({ remove: jest.fn() })),
}));

// Test component that uses theme sync
function TestThemeSyncComponent() {
  useThemeSync();
  const { theme, themeMode, isDarkMode } = useThemeContext();

  return (
    <View>
      <Text testID="themeMode">{themeMode}</Text>
      <Text testID="isDarkMode">{String(isDarkMode)}</Text>
      <Text testID="colorBackground">{theme.colors.background}</Text>
    </View>
  );
}

describe('useThemeSync', () => {
  beforeEach(() => {
    mockColorScheme = 'dark';
    // Reset settings store to initial state
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
  });

  it('loads settings on mount', async () => {
    const loadSettingsSpy = jest.fn();
    useSettingsStore.setState({ loadSettings: loadSettingsSpy });

    render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    await waitFor(() => {
      expect(loadSettingsSpy).toHaveBeenCalled();
    });
  });

  it('syncs theme when settings are loaded', async () => {
    // Start with dark theme setting
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'dark',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider initialThemeMode="light">
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    // After sync, the theme mode should be 'dark' from settings
    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('dark');
    });
  });

  it('uses system preference when theme is set to system', async () => {
    mockColorScheme = 'light';

    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'system',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('system');
      expect(getByTestId('isDarkMode').props.children).toBe('false');
    });
  });

  it('updates theme when settings change after load', async () => {
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'dark',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    // Initial theme is dark
    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('dark');
    });

    // Change theme setting to light
    act(() => {
      useSettingsStore.setState({
        settings: {
          units: 'metric',
          keepScreenAwake: true,
          hapticFeedback: 'medium',
          theme: 'light',
        },
        isLoaded: true,
        isSaving: false,
      });
    });

    // Theme should update to light
    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('light');
    });
  });

  it('applies dark theme colors when theme is dark', async () => {
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'dark',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    await waitFor(() => {
      expect(getByTestId('isDarkMode').props.children).toBe('true');
      // Dark theme background color
      expect(getByTestId('colorBackground').props.children).toBe('#121218');
    });
  });

  it('applies light theme colors when theme is light', async () => {
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'light',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    await waitFor(() => {
      expect(getByTestId('isDarkMode').props.children).toBe('false');
      // Light theme background color
      expect(getByTestId('colorBackground').props.children).toBe('#FAFAFC');
    });
  });
});

describe('useThemeSync with SettingsScreen integration', () => {
  beforeEach(() => {
    mockColorScheme = 'dark';
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
  });

  it('theme syncs immediately when settings are already loaded', async () => {
    // Simulate already loaded settings with light theme
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'light',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    // Theme should be synced from settings immediately
    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('light');
      expect(getByTestId('isDarkMode').props.children).toBe('false');
    });
  });

  it('handles theme change from system to manual mode', async () => {
    useSettingsStore.setState({
      settings: {
        units: 'metric',
        keepScreenAwake: true,
        hapticFeedback: 'medium',
        theme: 'system',
      },
      isLoaded: true,
      isSaving: false,
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <TestThemeSyncComponent />
      </ThemeProvider>,
    );

    // Initially following system (dark)
    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('system');
      expect(getByTestId('isDarkMode').props.children).toBe('true');
    });

    // Change to manual dark mode
    act(() => {
      useSettingsStore.setState({
        settings: {
          units: 'metric',
          keepScreenAwake: true,
          hapticFeedback: 'medium',
          theme: 'dark',
        },
        isLoaded: true,
        isSaving: false,
      });
    });

    await waitFor(() => {
      expect(getByTestId('themeMode').props.children).toBe('dark');
    });
  });
});
