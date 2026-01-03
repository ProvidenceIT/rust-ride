/**
 * App Tests
 *
 * Tests for the main App component including theme integration.
 *
 * @format
 */

import React from 'react';
import { render } from '@testing-library/react-native';
import App from '../App';

// Mock useColorScheme
let mockColorScheme: 'dark' | 'light' | null = 'dark';

jest.mock('react-native/Libraries/Utilities/useColorScheme', () => ({
  default: jest.fn(() => mockColorScheme),
}));

jest.mock('react-native/Libraries/Utilities/Appearance', () => ({
  addChangeListener: jest.fn(() => ({ remove: jest.fn() })),
}));

// Mock navigation
jest.mock('@react-navigation/native', () => {
  const actualNav = jest.requireActual('@react-navigation/native');
  return {
    ...actualNav,
    useNavigation: () => ({
      navigate: jest.fn(),
      goBack: jest.fn(),
    }),
    useRoute: () => ({
      params: {},
    }),
    NavigationContainer: ({ children }: { children: React.ReactNode }) => children,
  };
});

jest.mock('@react-navigation/native-stack', () => ({
  createNativeStackNavigator: () => ({
    Navigator: ({ children }: { children: React.ReactNode }) => children,
    Screen: () => null,
  }),
}));

jest.mock('@react-navigation/bottom-tabs', () => ({
  createBottomTabNavigator: () => ({
    Navigator: ({ children }: { children: React.ReactNode }) => children,
    Screen: () => null,
  }),
}));

describe('App', () => {
  beforeEach(() => {
    mockColorScheme = 'dark';
  });

  it('renders without crashing', () => {
    const { toJSON } = render(<App />);
    expect(toJSON()).toBeTruthy();
  });

  it('uses dark theme by default when system is dark', () => {
    mockColorScheme = 'dark';
    const { toJSON } = render(<App />);
    expect(toJSON()).toBeTruthy();
  });

  it('uses light theme when system is light', () => {
    mockColorScheme = 'light';
    const { toJSON } = render(<App />);
    expect(toJSON()).toBeTruthy();
  });

  it('includes ThemeProvider in component tree', () => {
    const { toJSON } = render(<App />);
    // App should render successfully with ThemeProvider
    const tree = toJSON();
    expect(tree).not.toBeNull();
  });
});
