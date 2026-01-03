/**
 * UI Components Tests
 *
 * Tests for the reusable UI components.
 */

import React from 'react';
import { Text } from 'react-native';
import { render } from '@testing-library/react-native';
import {
  MetricCard,
  Button,
  IconButton,
  ConnectionStatus,
  LoadingSpinner,
  FullScreenLoader,
  InlineLoader,
} from '../../src/components';
import { ThemeProvider } from '../../src/theme';

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement) => {
  return render(<ThemeProvider>{component}</ThemeProvider>);
};

// Mock icon component for IconButton tests
const MockIcon = ({ size, color }: { size?: number; color?: string }) => (
  <Text testID="mock-icon">{`Icon: ${size}x${color}`}</Text>
);

describe('MetricCard', () => {
  it('renders value and label', () => {
    const { getByText } = renderWithTheme(
      <MetricCard value={250} unit="W" label="Power" />,
    );

    expect(getByText('250')).toBeTruthy();
    expect(getByText('W')).toBeTruthy();
    expect(getByText('POWER')).toBeTruthy();
  });

  it('renders string values', () => {
    const { getByText } = renderWithTheme(
      <MetricCard value="--" unit="bpm" label="Heart Rate" />,
    );

    expect(getByText('--')).toBeTruthy();
    expect(getByText('bpm')).toBeTruthy();
  });

  it('renders with different sizes', () => {
    const { getByText: getSmall } = renderWithTheme(
      <MetricCard value={100} label="Small" size="small" />,
    );
    expect(getSmall('100')).toBeTruthy();

    const { getByText: getMedium } = renderWithTheme(
      <MetricCard value={200} label="Medium" size="medium" />,
    );
    expect(getMedium('200')).toBeTruthy();

    const { getByText: getLarge } = renderWithTheme(
      <MetricCard value={300} label="Large" size="large" />,
    );
    expect(getLarge('300')).toBeTruthy();
  });

  it('renders secondary value', () => {
    const { getByText } = renderWithTheme(
      <MetricCard
        value={250}
        label="Power"
        secondaryValue={245}
        secondaryLabel="3s avg"
      />,
    );

    expect(getByText('250')).toBeTruthy();
    expect(getByText('245')).toBeTruthy();
    expect(getByText('3s avg')).toBeTruthy();
  });

  it('renders target value', () => {
    const { getByText } = renderWithTheme(
      <MetricCard value={250} unit="W" label="Power" targetValue={260} />,
    );

    expect(getByText('250')).toBeTruthy();
    expect(getByText(/Target:/)).toBeTruthy();
    expect(getByText(/260/)).toBeTruthy();
  });

  it('has correct accessibility label', () => {
    const { getByLabelText } = renderWithTheme(
      <MetricCard value={180} unit="bpm" label="Heart Rate" />,
    );

    expect(getByLabelText('Heart Rate: 180 bpm')).toBeTruthy();
  });
});

describe('Button', () => {
  it('renders title', () => {
    const { getByText } = renderWithTheme(<Button title="Press Me" />);

    expect(getByText('Press Me')).toBeTruthy();
  });

  it('renders with different variants', () => {
    const variants = ['primary', 'secondary', 'outline', 'danger', 'ghost'] as const;

    variants.forEach(variant => {
      const { getByText } = renderWithTheme(<Button title={variant} variant={variant} />);
      expect(getByText(variant)).toBeTruthy();
    });
  });

  it('renders with different sizes', () => {
    const sizes = ['small', 'medium', 'large'] as const;

    sizes.forEach(size => {
      const { getByText } = renderWithTheme(<Button title={size} size={size} />);
      expect(getByText(size)).toBeTruthy();
    });
  });

  it('shows loading indicator when loading', () => {
    const { queryByText, UNSAFE_getByType } = renderWithTheme(
      <Button title="Submit" loading />,
    );

    // Title should not be visible when loading
    expect(queryByText('Submit')).toBeNull();
    // Activity indicator should be present
    expect(UNSAFE_getByType(require('react-native').ActivityIndicator)).toBeTruthy();
  });

  it('has correct accessibility state when disabled', () => {
    const { getByRole } = renderWithTheme(<Button title="Disabled" disabled />);

    const button = getByRole('button');
    expect(button.props.accessibilityState.disabled).toBe(true);
  });

  it('has correct accessibility state when loading', () => {
    const { getByRole } = renderWithTheme(<Button title="Loading" loading />);

    const button = getByRole('button');
    expect(button.props.accessibilityState.busy).toBe(true);
  });
});

describe('IconButton', () => {
  it('renders icon', () => {
    const { getByTestId } = renderWithTheme(
      <IconButton icon={<MockIcon />} accessibilityLabel="Test button" />,
    );

    expect(getByTestId('mock-icon')).toBeTruthy();
  });

  it('shows loading indicator when loading', () => {
    const { queryByTestId, UNSAFE_getByType } = renderWithTheme(
      <IconButton icon={<MockIcon />} accessibilityLabel="Loading button" loading />,
    );

    // Icon should not be visible when loading
    expect(queryByTestId('mock-icon')).toBeNull();
    // Activity indicator should be present
    expect(UNSAFE_getByType(require('react-native').ActivityIndicator)).toBeTruthy();
  });

  it('has correct accessibility label', () => {
    const { getByLabelText } = renderWithTheme(
      <IconButton icon={<MockIcon />} accessibilityLabel="Play workout" />,
    );

    expect(getByLabelText('Play workout')).toBeTruthy();
  });
});

describe('ConnectionStatus', () => {
  it('renders disconnected status', () => {
    const { getByText } = renderWithTheme(
      <ConnectionStatus status="disconnected" variant="badge" />,
    );

    expect(getByText('Disconnected')).toBeTruthy();
  });

  it('renders connecting status', () => {
    const { getByText } = renderWithTheme(
      <ConnectionStatus status="connecting" variant="badge" />,
    );

    expect(getByText('Connecting...')).toBeTruthy();
  });

  it('renders connected status', () => {
    const { getByText } = renderWithTheme(
      <ConnectionStatus status="connected" variant="badge" />,
    );

    expect(getByText('Connected')).toBeTruthy();
  });

  it('renders authenticated status', () => {
    const { getByText } = renderWithTheme(
      <ConnectionStatus status="authenticated" variant="badge" />,
    );

    expect(getByText('Authenticated')).toBeTruthy();
  });

  it('renders full variant with server name', () => {
    const { getByText } = renderWithTheme(
      <ConnectionStatus
        status="connected"
        variant="full"
        serverName="RustRide-PC:9876"
      />,
    );

    expect(getByText('Connected')).toBeTruthy();
    expect(getByText('RustRide-PC:9876')).toBeTruthy();
  });

  it('has correct accessibility label', () => {
    const { getByLabelText } = renderWithTheme(
      <ConnectionStatus status="connected" variant="badge" />,
    );

    expect(getByLabelText('Connected to server')).toBeTruthy();
  });
});

describe('LoadingSpinner', () => {
  it('renders without message', () => {
    const { UNSAFE_getByType } = renderWithTheme(<LoadingSpinner />);

    expect(UNSAFE_getByType(require('react-native').ActivityIndicator)).toBeTruthy();
  });

  it('renders with message', () => {
    const { getByText } = renderWithTheme(
      <LoadingSpinner message="Loading data..." />,
    );

    expect(getByText('Loading data...')).toBeTruthy();
  });

  it('renders different sizes', () => {
    const sizes = ['small', 'medium', 'large'] as const;

    sizes.forEach(size => {
      const { UNSAFE_getByType } = renderWithTheme(<LoadingSpinner size={size} />);
      expect(UNSAFE_getByType(require('react-native').ActivityIndicator)).toBeTruthy();
    });
  });

  it('has correct accessibility state', () => {
    const { getByRole } = renderWithTheme(<LoadingSpinner message="Loading" />);

    const progressbar = getByRole('progressbar');
    expect(progressbar.props.accessibilityState.busy).toBe(true);
  });
});

describe('FullScreenLoader', () => {
  it('renders with message', () => {
    const { getByText } = renderWithTheme(<FullScreenLoader message="Please wait..." />);

    expect(getByText('Please wait...')).toBeTruthy();
  });
});

describe('InlineLoader', () => {
  it('renders small spinner', () => {
    const { UNSAFE_getByType } = renderWithTheme(<InlineLoader />);

    expect(UNSAFE_getByType(require('react-native').ActivityIndicator)).toBeTruthy();
  });
});
