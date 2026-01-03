/**
 * EmptyState Component Tests
 */

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { EmptyState, CompactEmptyState } from '../EmptyState';

// Mock the theme hook
jest.mock('@/theme', () => ({
  useTheme: () => ({
    colors: {
      accent: '#FF6B00',
      textPrimary: '#FFFFFF',
      textSecondary: '#A0A0A0',
      textMuted: '#666666',
      textInverse: '#FFFFFF',
      surface: '#2A2A2A',
      card: '#1A1A1A',
      error: '#FF4444',
      warning: '#FFAA00',
    },
    spacing: {
      xs: 4,
      sm: 8,
      md: 16,
      lg: 24,
      xl: 32,
    },
    typography: {
      textStyles: {
        sectionTitle: { fontSize: 18, fontWeight: '600' },
        body: { fontSize: 14 },
      },
    },
    borderRadius: {
      md: 8,
      lg: 12,
      full: 9999,
    },
  }),
}));

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

// Mock Button component
jest.mock('../Button', () => ({
  Button: ({
    title,
    onPress,
    loading,
    disabled,
    testID,
  }: {
    title: string;
    onPress?: () => void;
    loading?: boolean;
    disabled?: boolean;
    testID?: string;
  }) => {
    const { TouchableOpacity, Text } = require('react-native');
    return (
      <TouchableOpacity
        onPress={onPress}
        disabled={disabled}
        testID={testID}
        accessibilityRole="button"
      >
        <Text>{loading ? 'Loading...' : title}</Text>
      </TouchableOpacity>
    );
  },
}));

describe('EmptyState', () => {
  describe('Predefined Variants', () => {
    it('renders no-rides variant correctly', () => {
      const { getByText } = render(<EmptyState variant="no-rides" />);

      expect(getByText('No Rides Yet')).toBeTruthy();
      expect(getByText('Complete a ride on your desktop app and it will appear here.')).toBeTruthy();
    });

    it('renders no-connection variant correctly', () => {
      const { getByText } = render(<EmptyState variant="no-connection" />);

      expect(getByText('Not Connected')).toBeTruthy();
      expect(
        getByText('Connect to your RustRide desktop app to view data and control workouts.')
      ).toBeTruthy();
    });

    it('renders error variant correctly', () => {
      const { getByText } = render(<EmptyState variant="error" />);

      expect(getByText('Something Went Wrong')).toBeTruthy();
      expect(getByText('An error occurred while loading data. Please try again.')).toBeTruthy();
    });

    it('renders no-results variant correctly', () => {
      const { getByText } = render(<EmptyState variant="no-results" />);

      expect(getByText('No Matching Results')).toBeTruthy();
      expect(
        getByText('No items match your current filters. Try adjusting your filters or clear them.')
      ).toBeTruthy();
    });

    it('renders offline variant correctly', () => {
      const { getByText } = render(<EmptyState variant="offline" />);

      expect(getByText("You're Offline")).toBeTruthy();
      expect(getByText('Connect to the internet to sync your data.')).toBeTruthy();
    });

    it('renders loading-failed variant correctly', () => {
      const { getByText } = render(<EmptyState variant="loading-failed" />);

      expect(getByText('Failed to Load')).toBeTruthy();
      expect(
        getByText("We couldn't load the data. Please check your connection and try again.")
      ).toBeTruthy();
    });

    it('renders no-session variant correctly', () => {
      const { getByText } = render(<EmptyState variant="no-session" />);

      expect(getByText('Ready to Ride')).toBeTruthy();
      expect(
        getByText('Start a workout or free ride on the desktop app to see live metrics here.')
      ).toBeTruthy();
    });
  });

  describe('Custom Content', () => {
    it('allows overriding title', () => {
      const { getByText, queryByText } = render(
        <EmptyState variant="no-rides" title="Custom Title" />
      );

      expect(getByText('Custom Title')).toBeTruthy();
      expect(queryByText('No Rides Yet')).toBeNull();
    });

    it('allows overriding description', () => {
      const { getByText, queryByText } = render(
        <EmptyState variant="no-rides" description="Custom description text" />
      );

      expect(getByText('Custom description text')).toBeTruthy();
      expect(queryByText('Complete a ride on your desktop app and it will appear here.')).toBeNull();
    });

    it('renders custom variant with provided content', () => {
      const { getByText } = render(
        <EmptyState
          variant="custom"
          icon="search-outline"
          title="No Search Results"
          description="Try a different search term"
        />
      );

      expect(getByText('No Search Results')).toBeTruthy();
      expect(getByText('Try a different search term')).toBeTruthy();
    });

    it('renders additional children', () => {
      const { Text } = require('react-native');
      const { getByText } = render(
        <EmptyState variant="no-rides">
          <Text>Additional content</Text>
        </EmptyState>
      );

      // Title should still be there
      expect(getByText('No Rides Yet')).toBeTruthy();
      expect(getByText('Additional content')).toBeTruthy();
    });
  });

  describe('Action Buttons', () => {
    it('renders primary action button when provided', () => {
      const handleAction = jest.fn();
      const { getByText } = render(
        <EmptyState
          variant="no-connection"
          actionLabel="Connect Now"
          onAction={handleAction}
        />
      );

      const button = getByText('Connect Now');
      expect(button).toBeTruthy();

      fireEvent.press(button);
      expect(handleAction).toHaveBeenCalledTimes(1);
    });

    it('renders secondary action button when provided', () => {
      const handlePrimary = jest.fn();
      const handleSecondary = jest.fn();
      const { getByText } = render(
        <EmptyState
          variant="error"
          actionLabel="Try Again"
          onAction={handlePrimary}
          secondaryActionLabel="Go Back"
          onSecondaryAction={handleSecondary}
        />
      );

      const primaryButton = getByText('Try Again');
      const secondaryButton = getByText('Go Back');

      expect(primaryButton).toBeTruthy();
      expect(secondaryButton).toBeTruthy();

      fireEvent.press(primaryButton);
      expect(handlePrimary).toHaveBeenCalledTimes(1);

      fireEvent.press(secondaryButton);
      expect(handleSecondary).toHaveBeenCalledTimes(1);
    });

    it('shows loading state on action button', () => {
      const { getByText } = render(
        <EmptyState
          variant="error"
          actionLabel="Try Again"
          onAction={jest.fn()}
          isActionLoading={true}
        />
      );

      expect(getByText('Loading...')).toBeTruthy();
    });

    it('uses variant default action label when not overridden', () => {
      const handleAction = jest.fn();
      const { getByText } = render(
        <EmptyState variant="no-connection" onAction={handleAction} />
      );

      // Default action label for no-connection is "Connect"
      expect(getByText('Connect')).toBeTruthy();
    });
  });

  describe('Accessibility', () => {
    it('has correct accessibility attributes', () => {
      const { getByTestId } = render(<EmptyState variant="no-rides" testID="empty-state" />);

      const container = getByTestId('empty-state');
      expect(container.props.accessibilityRole).toBe('text');
    });

    it('includes title and description in accessibility label', () => {
      const { getByLabelText } = render(<EmptyState variant="no-rides" />);

      expect(
        getByLabelText(
          'No Rides Yet. Complete a ride on your desktop app and it will appear here.'
        )
      ).toBeTruthy();
    });
  });

  describe('testID', () => {
    it('passes testID to container', () => {
      const { getByTestId } = render(
        <EmptyState variant="no-rides" testID="empty-state-test" />
      );

      expect(getByTestId('empty-state-test')).toBeTruthy();
    });
  });
});

describe('CompactEmptyState', () => {
  it('renders with message', () => {
    const { getByText } = render(
      <CompactEmptyState message="No items found" />
    );

    expect(getByText('No items found')).toBeTruthy();
  });

  it('renders action button when provided', () => {
    const handleAction = jest.fn();
    const { getByText } = render(
      <CompactEmptyState
        message="No items found"
        actionLabel="Clear"
        onAction={handleAction}
      />
    );

    const button = getByText('Clear');
    expect(button).toBeTruthy();

    fireEvent.press(button);
    expect(handleAction).toHaveBeenCalledTimes(1);
  });

  it('has correct accessibility attributes', () => {
    const { getByLabelText } = render(
      <CompactEmptyState message="No search results" />
    );

    expect(getByLabelText('No search results')).toBeTruthy();
  });

  it('passes testID to container', () => {
    const { getByTestId } = render(
      <CompactEmptyState message="Test message" testID="compact-empty-test" />
    );

    expect(getByTestId('compact-empty-test')).toBeTruthy();
  });

  it('renders with custom icon', () => {
    const { getByText } = render(
      <CompactEmptyState
        icon="search-outline"
        message="No search results"
      />
    );

    // Icon is mocked, just verify message renders
    expect(getByText('No search results')).toBeTruthy();
  });
});
