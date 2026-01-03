/**
 * ThemeProvider Tests
 *
 * Tests for the ThemeProvider component and hooks.
 */

// Mock useColorScheme before importing
let mockColorScheme: 'dark' | 'light' | null = 'dark';

jest.mock('react-native/Libraries/Utilities/useColorScheme', () => ({
  default: jest.fn(() => mockColorScheme),
}));

jest.mock('react-native/Libraries/Utilities/Appearance', () => ({
  addChangeListener: jest.fn(() => ({ remove: jest.fn() })),
}));

import React from 'react';
import { Text } from 'react-native';
import { render, act } from '@testing-library/react-native';
import {
  ThemeProvider,
  useTheme,
  useThemeContext,
  useIsDarkMode,
  useColors,
  darkTheme,
  lightTheme,
} from '@theme';

// Test component to access theme hooks
function TestComponent({ testId }: { testId?: string }) {
  const theme = useTheme();
  return <Text testID={testId}>{theme.mode}</Text>;
}

// Test component to access theme context
function TestContextComponent() {
  const { theme, themeMode, isDarkMode, toggleTheme } = useThemeContext();
  return (
    <>
      <Text testID="mode">{theme.mode}</Text>
      <Text testID="themeMode">{themeMode}</Text>
      <Text testID="isDarkMode">{String(isDarkMode)}</Text>
      <Text testID="toggle" onPress={toggleTheme}>
        Toggle
      </Text>
    </>
  );
}

// Test component for useIsDarkMode
function TestIsDarkModeComponent() {
  const isDarkMode = useIsDarkMode();
  return <Text testID="isDarkMode">{String(isDarkMode)}</Text>;
}

// Test component for useColors
function TestColorsComponent() {
  const colors = useColors();
  return <Text testID="background">{colors.background}</Text>;
}

describe('ThemeProvider', () => {
  beforeEach(() => {
    mockColorScheme = 'dark';
  });

  it('provides dark theme by default when system is dark', () => {
    const { getByTestId } = render(
      <ThemeProvider>
        <TestComponent testId="theme" />
      </ThemeProvider>,
    );

    expect(getByTestId('theme').props.children).toBe('dark');
  });

  it('provides light theme when system is light', () => {
    mockColorScheme = 'light';

    const { getByTestId } = render(
      <ThemeProvider>
        <TestComponent testId="theme" />
      </ThemeProvider>,
    );

    expect(getByTestId('theme').props.children).toBe('light');
  });

  it('uses initial theme mode when provided', () => {
    const { getByTestId } = render(
      <ThemeProvider initialThemeMode="light">
        <TestComponent testId="theme" />
      </ThemeProvider>,
    );

    expect(getByTestId('theme').props.children).toBe('light');
  });

  it('allows overriding system preference with explicit mode', () => {
    mockColorScheme = 'light';

    const { getByTestId } = render(
      <ThemeProvider initialThemeMode="dark">
        <TestComponent testId="theme" />
      </ThemeProvider>,
    );

    expect(getByTestId('theme').props.children).toBe('dark');
  });
});

describe('useThemeContext', () => {
  beforeEach(() => {
    mockColorScheme = 'dark';
  });

  it('provides theme context with all values', () => {
    const { getByTestId } = render(
      <ThemeProvider>
        <TestContextComponent />
      </ThemeProvider>,
    );

    expect(getByTestId('mode').props.children).toBe('dark');
    expect(getByTestId('themeMode').props.children).toBe('system');
    expect(getByTestId('isDarkMode').props.children).toBe('true');
  });

  it('toggleTheme switches from dark to light', () => {
    const { getByTestId } = render(
      <ThemeProvider>
        <TestContextComponent />
      </ThemeProvider>,
    );

    expect(getByTestId('mode').props.children).toBe('dark');

    act(() => {
      getByTestId('toggle').props.onPress();
    });

    expect(getByTestId('mode').props.children).toBe('light');
  });
});

describe('useIsDarkMode', () => {
  it('returns true when in dark mode', () => {
    mockColorScheme = 'dark';

    const { getByTestId } = render(
      <ThemeProvider>
        <TestIsDarkModeComponent />
      </ThemeProvider>,
    );

    expect(getByTestId('isDarkMode').props.children).toBe('true');
  });

  it('returns false when in light mode', () => {
    mockColorScheme = 'light';

    const { getByTestId } = render(
      <ThemeProvider>
        <TestIsDarkModeComponent />
      </ThemeProvider>,
    );

    expect(getByTestId('isDarkMode').props.children).toBe('false');
  });
});

describe('useColors', () => {
  it('returns dark colors when in dark mode', () => {
    mockColorScheme = 'dark';

    const { getByTestId } = render(
      <ThemeProvider>
        <TestColorsComponent />
      </ThemeProvider>,
    );

    expect(getByTestId('background').props.children).toBe(darkTheme.colors.background);
  });

  it('returns light colors when in light mode', () => {
    mockColorScheme = 'light';

    const { getByTestId } = render(
      <ThemeProvider>
        <TestColorsComponent />
      </ThemeProvider>,
    );

    expect(getByTestId('background').props.children).toBe(lightTheme.colors.background);
  });
});
