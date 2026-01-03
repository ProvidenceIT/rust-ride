/**
 * UI Components Tests
 *
 * Tests for the reusable UI components.
 */

import React from 'react';
import { Text } from 'react-native';
import { render, fireEvent } from '@testing-library/react-native';
import {
  MetricCard,
  Button,
  IconButton,
  ConnectionStatus,
  LoadingSpinner,
  FullScreenLoader,
  InlineLoader,
  ServerListItem,
  ManualEntryModal,
  QRScannerModal,
  PinEntryModal,
  PowerDisplay,
  HeartRateDisplay,
} from '../../src/components';
import {
  parseQrConnectionData,
  parseWebSocketUrl,
} from '../../src/types';
import {
  setMockPermissionStatus,
  resetMocks as resetCameraMocks,
} from '../../__mocks__/react-native-camera-kit';
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

describe('ServerListItem', () => {
  const mockServer = {
    name: 'RustRide-PC',
    host: '192.168.1.100',
    port: 9876,
    version: '1.0',
  };

  const mockOnPress = jest.fn();

  beforeEach(() => {
    mockOnPress.mockClear();
  });

  it('renders server name', () => {
    const { getByText } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} />,
    );

    expect(getByText('RustRide-PC')).toBeTruthy();
  });

  it('renders host and port', () => {
    const { getByText } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} />,
    );

    expect(getByText('192.168.1.100:9876')).toBeTruthy();
  });

  it('renders version badge when version is provided', () => {
    const { getByText } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} />,
    );

    expect(getByText('v1.0')).toBeTruthy();
  });

  it('does not render version badge when version is not provided', () => {
    const serverWithoutVersion = { ...mockServer, version: undefined };
    const { queryByText } = renderWithTheme(
      <ServerListItem server={serverWithoutVersion} onPress={mockOnPress} />,
    );

    expect(queryByText(/^v/)).toBeNull();
  });

  it('calls onPress when pressed', () => {
    const { getByRole } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} />,
    );

    const button = getByRole('button');
    fireEvent.press(button);

    expect(mockOnPress).toHaveBeenCalledWith(mockServer);
  });

  it('does not call onPress when connecting', () => {
    const { getByRole } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} isConnecting />,
    );

    const button = getByRole('button');
    fireEvent.press(button);

    expect(mockOnPress).not.toHaveBeenCalled();
  });

  it('shows connecting text when isConnecting is true', () => {
    const { getByText } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} isConnecting />,
    );

    expect(getByText('Connecting...')).toBeTruthy();
  });

  it('has correct accessibility label', () => {
    const { getByLabelText } = renderWithTheme(
      <ServerListItem server={mockServer} onPress={mockOnPress} />,
    );

    expect(getByLabelText('Connect to RustRide-PC at 192.168.1.100:9876')).toBeTruthy();
  });
});

describe('ManualEntryModal', () => {
  const mockOnClose = jest.fn();
  const mockOnSubmit = jest.fn();

  beforeEach(() => {
    mockOnClose.mockClear();
    mockOnSubmit.mockClear();
  });

  it('renders when visible', () => {
    const { getByText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByText('Manual Connection')).toBeTruthy();
  });

  it('does not render when not visible', () => {
    const { queryByText } = renderWithTheme(
      <ManualEntryModal visible={false} onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(queryByText('Manual Connection')).toBeNull();
  });

  it('renders IP address and port input fields', () => {
    const { getByText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByText('IP Address or Hostname')).toBeTruthy();
    expect(getByText('Port')).toBeTruthy();
  });

  it('renders connect and cancel buttons', () => {
    const { getByText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByText('Connect')).toBeTruthy();
    expect(getByText('Cancel')).toBeTruthy();
  });

  it('calls onClose when cancel is pressed', () => {
    const { getByText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.press(getByText('Cancel'));

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('shows error for empty IP address on submit', () => {
    const { getByText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.press(getByText('Connect'));

    expect(getByText('IP address is required')).toBeTruthy();
  });

  it('shows error for invalid IP address on submit', () => {
    const { getByText, getByPlaceholderText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.changeText(getByPlaceholderText('192.168.1.100'), 'invalid..ip');
    fireEvent.press(getByText('Connect'));

    expect(getByText('Invalid IP address or hostname')).toBeTruthy();
  });

  it('calls onSubmit with valid server data', () => {
    const { getByText, getByPlaceholderText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.changeText(getByPlaceholderText('192.168.1.100'), '192.168.1.50');
    fireEvent.changeText(getByPlaceholderText('9876'), '8080');
    fireEvent.press(getByText('Connect'));

    expect(mockOnSubmit).toHaveBeenCalledWith({
      name: 'Manual (192.168.1.50)',
      host: '192.168.1.50',
      port: 8080,
    });
  });

  it('accepts valid hostname', () => {
    const { getByText, getByPlaceholderText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.changeText(getByPlaceholderText('192.168.1.100'), 'my-computer.local');
    fireEvent.press(getByText('Connect'));

    expect(mockOnSubmit).toHaveBeenCalledWith({
      name: 'Manual (my-computer.local)',
      host: 'my-computer.local',
      port: 9876,
    });
  });

  it('shows error for invalid port', () => {
    const { getByText, getByPlaceholderText } = renderWithTheme(
      <ManualEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.changeText(getByPlaceholderText('192.168.1.100'), '192.168.1.50');
    fireEvent.changeText(getByPlaceholderText('9876'), '99999');
    fireEvent.press(getByText('Connect'));

    expect(getByText('Invalid port (1-65535)')).toBeTruthy();
  });
});

describe('parseQrConnectionData', () => {
  it('parses valid QR code with PIN', () => {
    const qrData = JSON.stringify({
      url: 'ws://192.168.1.100:9876',
      pin: '123456',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toEqual({
      url: 'ws://192.168.1.100:9876',
      pin: '123456',
      version: '1',
    });
  });

  it('parses valid QR code without PIN', () => {
    const qrData = JSON.stringify({
      url: 'ws://192.168.1.100:9876',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toEqual({
      url: 'ws://192.168.1.100:9876',
      pin: undefined,
      version: '1',
    });
  });

  it('parses wss:// URL', () => {
    const qrData = JSON.stringify({
      url: 'wss://example.com:443',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).not.toBeNull();
    expect(result?.url).toBe('wss://example.com:443');
  });

  it('returns null for invalid JSON', () => {
    const result = parseQrConnectionData('not valid json');

    expect(result).toBeNull();
  });

  it('returns null for missing URL', () => {
    const qrData = JSON.stringify({
      pin: '123456',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toBeNull();
  });

  it('returns null for missing version', () => {
    const qrData = JSON.stringify({
      url: 'ws://192.168.1.100:9876',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toBeNull();
  });

  it('returns null for invalid URL scheme', () => {
    const qrData = JSON.stringify({
      url: 'http://192.168.1.100:9876',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toBeNull();
  });

  it('returns null for invalid PIN format (too short)', () => {
    const qrData = JSON.stringify({
      url: 'ws://192.168.1.100:9876',
      pin: '123',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toBeNull();
  });

  it('returns null for invalid PIN format (non-numeric)', () => {
    const qrData = JSON.stringify({
      url: 'ws://192.168.1.100:9876',
      pin: 'abcdef',
      version: '1',
    });

    const result = parseQrConnectionData(qrData);

    expect(result).toBeNull();
  });
});

describe('parseWebSocketUrl', () => {
  it('parses URL with port', () => {
    const result = parseWebSocketUrl('ws://192.168.1.100:9876');

    expect(result).toEqual({
      host: '192.168.1.100',
      port: 9876,
    });
  });

  it('parses URL without port (uses default)', () => {
    const result = parseWebSocketUrl('ws://192.168.1.100');

    expect(result).toEqual({
      host: '192.168.1.100',
      port: 9876,
    });
  });

  it('parses URL with hostname', () => {
    const result = parseWebSocketUrl('ws://my-computer.local:8080');

    expect(result).toEqual({
      host: 'my-computer.local',
      port: 8080,
    });
  });

  it('parses wss:// URL', () => {
    const result = parseWebSocketUrl('wss://example.com:443');

    expect(result).toEqual({
      host: 'example.com',
      port: 443,
    });
  });

  it('returns null for invalid URL', () => {
    const result = parseWebSocketUrl('not a valid url');

    expect(result).toBeNull();
  });

  it('returns null for empty string', () => {
    const result = parseWebSocketUrl('');

    expect(result).toBeNull();
  });
});

describe('QRScannerModal', () => {
  const mockOnClose = jest.fn();
  const mockOnScan = jest.fn();

  // Helper to wait for async operations
  const wait = (ms: number): Promise<void> =>
    new Promise<void>(resolve => setTimeout(resolve, ms));

  beforeEach(() => {
    mockOnClose.mockClear();
    mockOnScan.mockClear();
    resetCameraMocks();
    setMockPermissionStatus(true);
  });

  it('renders when visible with camera permission granted', async () => {
    const { getByText } = renderWithTheme(
      <QRScannerModal visible onClose={mockOnClose} onScan={mockOnScan} />,
    );

    // Wait for permission check
    await wait(100);

    expect(getByText('Scan QR Code')).toBeTruthy();
  });

  it('does not render content when not visible', () => {
    const { queryByText } = renderWithTheme(
      <QRScannerModal visible={false} onClose={mockOnClose} onScan={mockOnScan} />,
    );

    expect(queryByText('Scan QR Code')).toBeNull();
  });

  it('shows permission denied message when camera access denied', async () => {
    setMockPermissionStatus(false);

    const { getByText } = renderWithTheme(
      <QRScannerModal visible onClose={mockOnClose} onScan={mockOnScan} />,
    );

    // Wait for permission check
    await wait(100);

    expect(getByText('Camera Access Required')).toBeTruthy();
  });

  it('calls onClose when cancel is pressed', async () => {
    const { getByText } = renderWithTheme(
      <QRScannerModal visible onClose={mockOnClose} onScan={mockOnScan} />,
    );

    // Wait for permission check
    await wait(100);

    fireEvent.press(getByText('Cancel'));

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('renders instructions text', async () => {
    const { getByText } = renderWithTheme(
      <QRScannerModal visible onClose={mockOnClose} onScan={mockOnScan} />,
    );

    // Wait for permission check
    await wait(100);

    expect(getByText(/Point your camera at the QR code/)).toBeTruthy();
  });

  it('shows connecting state when isConnecting is true', async () => {
    const { getByText } = renderWithTheme(
      <QRScannerModal visible onClose={mockOnClose} onScan={mockOnScan} isConnecting />,
    );

    // Wait for permission check
    await wait(100);

    expect(getByText('Connecting...')).toBeTruthy();
  });
});

describe('PinEntryModal', () => {
  const mockOnClose = jest.fn();
  const mockOnSubmit = jest.fn();

  beforeEach(() => {
    mockOnClose.mockClear();
    mockOnSubmit.mockClear();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('renders when visible', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByText('Enter PIN')).toBeTruthy();
  });

  it('does not render content when not visible', () => {
    const { queryByText } = renderWithTheme(
      <PinEntryModal visible={false} onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(queryByText('Enter PIN')).toBeNull();
  });

  it('renders server name in instructions when provided', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal
        visible
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        serverName="RustRide-PC"
      />,
    );

    expect(getByText(/Enter the PIN shown on RustRide-PC/)).toBeTruthy();
  });

  it('renders default instructions when no server name', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByText(/Enter the PIN shown on the RustRide desktop app/)).toBeTruthy();
  });

  it('renders numeric keypad with digits 0-9', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    // Check that all digits are present
    for (let i = 0; i <= 9; i++) {
      expect(getByLabelText(String(i))).toBeTruthy();
    }
  });

  it('renders delete button', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByLabelText('Delete')).toBeTruthy();
  });

  it('renders cancel button', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    expect(getByText('Cancel')).toBeTruthy();
  });

  it('calls onClose when cancel is pressed', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.press(getByText('Cancel'));

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('updates PIN display when digits are pressed', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    fireEvent.press(getByLabelText('1'));
    fireEvent.press(getByLabelText('2'));
    fireEvent.press(getByLabelText('3'));

    // Check accessibility label updates
    expect(getByLabelText('PIN entry, 3 of 6 digits entered')).toBeTruthy();
  });

  it('calls onSubmit when 6 digits are entered', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    // Enter 6 digits
    fireEvent.press(getByLabelText('1'));
    fireEvent.press(getByLabelText('2'));
    fireEvent.press(getByLabelText('3'));
    fireEvent.press(getByLabelText('4'));
    fireEvent.press(getByLabelText('5'));
    fireEvent.press(getByLabelText('6'));

    expect(mockOnSubmit).toHaveBeenCalledWith('123456');
  });

  it('deletes last digit when delete is pressed', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    // Enter some digits
    fireEvent.press(getByLabelText('1'));
    fireEvent.press(getByLabelText('2'));
    fireEvent.press(getByLabelText('3'));

    // Delete one
    fireEvent.press(getByLabelText('Delete'));

    // Should now have 2 digits
    expect(getByLabelText('PIN entry, 2 of 6 digits entered')).toBeTruthy();
  });

  it('shows error message when error prop is set', () => {
    const { getByText, rerender } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    // Re-render with error
    rerender(
      <ThemeProvider>
        <PinEntryModal
          visible
          onClose={mockOnClose}
          onSubmit={mockOnSubmit}
          error="Invalid PIN"
        />
      </ThemeProvider>,
    );

    // Fast forward timers for animation
    jest.runAllTimers();

    expect(getByText('Invalid PIN')).toBeTruthy();
  });

  it('shows authenticating state when isAuthenticating is true', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal
        visible
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        isAuthenticating
      />,
    );

    expect(getByText('Authenticating...')).toBeTruthy();
  });

  it('disables digit buttons when authenticating', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal
        visible
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        isAuthenticating
      />,
    );

    // Try to enter a digit
    fireEvent.press(getByLabelText('1'));

    // Should still be 0 digits
    expect(getByLabelText('PIN entry, 0 of 6 digits entered')).toBeTruthy();
  });

  it('disables cancel button when authenticating', () => {
    const { getByText } = renderWithTheme(
      <PinEntryModal
        visible
        onClose={mockOnClose}
        onSubmit={mockOnSubmit}
        isAuthenticating
      />,
    );

    fireEvent.press(getByText('Cancel'));

    // onClose should not be called because button is disabled
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('does not allow more than 6 digits', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    // Enter 7 digits
    fireEvent.press(getByLabelText('1'));
    fireEvent.press(getByLabelText('2'));
    fireEvent.press(getByLabelText('3'));
    fireEvent.press(getByLabelText('4'));
    fireEvent.press(getByLabelText('5'));
    fireEvent.press(getByLabelText('6'));
    fireEvent.press(getByLabelText('7'));

    // Should only call onSubmit once with 6 digits
    expect(mockOnSubmit).toHaveBeenCalledTimes(1);
    expect(mockOnSubmit).toHaveBeenCalledWith('123456');
  });

  it('resets PIN when modal becomes visible', () => {
    const { rerender, getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    // Enter some digits
    fireEvent.press(getByLabelText('1'));
    fireEvent.press(getByLabelText('2'));

    // Hide modal
    rerender(
      <ThemeProvider>
        <PinEntryModal visible={false} onClose={mockOnClose} onSubmit={mockOnSubmit} />
      </ThemeProvider>,
    );

    // Show modal again
    rerender(
      <ThemeProvider>
        <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />
      </ThemeProvider>,
    );

    // Should be reset to 0 digits
    expect(getByLabelText('PIN entry, 0 of 6 digits entered')).toBeTruthy();
  });

  it('has correct accessibility for PIN display', () => {
    const { getByLabelText } = renderWithTheme(
      <PinEntryModal visible onClose={mockOnClose} onSubmit={mockOnSubmit} />,
    );

    const pinDisplay = getByLabelText('PIN entry, 0 of 6 digits entered');
    expect(pinDisplay.props.accessibilityRole).toBe('text');
    expect(pinDisplay.props.accessibilityLiveRegion).toBe('polite');
  });
});

describe('PowerDisplay', () => {
  it('renders current power value', () => {
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={250}
        power3sAvg={245}
        powerZone="tempo"
        showMetrics={true}
      />,
    );

    expect(getByText('250')).toBeTruthy();
    expect(getByText('W')).toBeTruthy();
    expect(getByText('POWER')).toBeTruthy();
  });

  it('renders 3-second average', () => {
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={280}
        power3sAvg={275}
        powerZone="threshold"
        showMetrics={true}
      />,
    );

    expect(getByText('275')).toBeTruthy();
    expect(getByText(/3s avg/)).toBeTruthy();
  });

  it('renders power zone badge', () => {
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={300}
        power3sAvg={295}
        powerZone="vo2max"
        showMetrics={true}
      />,
    );

    expect(getByText(/Z5 VO2max/i)).toBeTruthy();
  });

  it('renders different zone badges correctly', () => {
    const zones = [
      { zone: 'recovery', label: 'Z1 Recovery' },
      { zone: 'endurance', label: 'Z2 Endurance' },
      { zone: 'tempo', label: 'Z3 Tempo' },
      { zone: 'threshold', label: 'Z4 Threshold' },
      { zone: 'vo2max', label: 'Z5 VO2max' },
      { zone: 'anaerobic', label: 'Z6 Anaerobic' },
      { zone: 'neuromuscular', label: 'Z7 Neuromuscular' },
    ] as const;

    zones.forEach(({ zone, label }) => {
      const { getByText } = renderWithTheme(
        <PowerDisplay
          power={200}
          power3sAvg={195}
          powerZone={zone}
          showMetrics={true}
        />,
      );

      expect(getByText(label)).toBeTruthy();
    });
  });

  it('renders target power when provided', () => {
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={275}
        power3sAvg={270}
        powerZone="threshold"
        targetPower={280}
        showMetrics={true}
      />,
    );

    expect(getByText('TARGET')).toBeTruthy();
    expect(getByText('280')).toBeTruthy();
  });

  it('shows power difference from target', () => {
    // Power is 320W vs target 280W = +40W (14.3% above, outside 5% tolerance)
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={320}
        power3sAvg={315}
        powerZone="threshold"
        targetPower={280}
        showMetrics={true}
      />,
    );

    expect(getByText('+40W')).toBeTruthy();
    expect(getByText('Reduce power')).toBeTruthy();
  });

  it('shows negative power difference', () => {
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={260}
        power3sAvg={255}
        powerZone="tempo"
        targetPower={280}
        showMetrics={true}
      />,
    );

    expect(getByText('-20W')).toBeTruthy();
    expect(getByText('Increase power')).toBeTruthy();
  });

  it('shows on target message when within 5%', () => {
    const { getByText } = renderWithTheme(
      <PowerDisplay
        power={282}
        power3sAvg={280}
        powerZone="threshold"
        targetPower={280}
        showMetrics={true}
      />,
    );

    expect(getByText('On target')).toBeTruthy();
  });

  it('renders placeholder when showMetrics is false', () => {
    const { getAllByText, queryByText } = renderWithTheme(
      <PowerDisplay
        power={250}
        power3sAvg={245}
        powerZone="tempo"
        showMetrics={false}
      />,
    );

    // Both power and 3s avg show "--" when no metrics
    const placeholders = getAllByText('--');
    expect(placeholders.length).toBeGreaterThanOrEqual(1);
    expect(queryByText('250')).toBeNull();
    expect(queryByText(/Z3 Tempo/)).toBeNull();
  });

  it('does not show zone badge when power is 0', () => {
    const { queryByText } = renderWithTheme(
      <PowerDisplay
        power={0}
        power3sAvg={0}
        powerZone="recovery"
        showMetrics={true}
      />,
    );

    expect(queryByText(/Z1 Recovery/)).toBeNull();
  });

  it('does not show target section when targetPower is null', () => {
    const { queryByText } = renderWithTheme(
      <PowerDisplay
        power={250}
        power3sAvg={245}
        powerZone="tempo"
        targetPower={null}
        showMetrics={true}
      />,
    );

    expect(queryByText('TARGET')).toBeNull();
  });

  it('does not show target section when showMetrics is false', () => {
    const { queryByText } = renderWithTheme(
      <PowerDisplay
        power={250}
        power3sAvg={245}
        powerZone="tempo"
        targetPower={280}
        showMetrics={false}
      />,
    );

    expect(queryByText('TARGET')).toBeNull();
  });

  it('has correct accessibility label', () => {
    const { getByLabelText } = renderWithTheme(
      <PowerDisplay
        power={285}
        power3sAvg={280}
        powerZone="threshold"
        targetPower={275}
        showMetrics={true}
      />,
    );

    expect(
      getByLabelText(/Power: 285 watts.*3 second average: 280 watts.*Zone: Threshold.*Target: 275 watts/),
    ).toBeTruthy();
  });

  it('has correct accessibility label without target', () => {
    const { getByLabelText } = renderWithTheme(
      <PowerDisplay
        power={200}
        power3sAvg={195}
        powerZone="endurance"
        showMetrics={true}
      />,
    );

    expect(
      getByLabelText(/Power: 200 watts.*3 second average: 195 watts.*Zone: Endurance/),
    ).toBeTruthy();
  });

  it('has correct accessibility label when no metrics', () => {
    const { getByLabelText } = renderWithTheme(
      <PowerDisplay
        power={0}
        power3sAvg={0}
        powerZone="recovery"
        showMetrics={false}
      />,
    );

    expect(
      getByLabelText(/Power: no data/),
    ).toBeTruthy();
  });
});

describe('HeartRateDisplay', () => {
  it('renders current heart rate value', () => {
    const { getByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={145}
        hrZone="zone4"
        maxHeartRate={165}
        showMetrics={true}
      />,
    );

    expect(getByText('145')).toBeTruthy();
    expect(getByText('bpm')).toBeTruthy();
    expect(getByText('HEART RATE')).toBeTruthy();
  });

  it('renders max heart rate', () => {
    const { getByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={145}
        hrZone="zone4"
        maxHeartRate={170}
        showMetrics={true}
      />,
    );

    expect(getByText('170')).toBeTruthy();
    expect(getByText(/max/)).toBeTruthy();
  });

  it('renders HR zone badge', () => {
    const { getByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={160}
        hrZone="zone5"
        maxHeartRate={165}
        showMetrics={true}
      />,
    );

    expect(getByText(/Z5 Max/)).toBeTruthy();
  });

  it('renders different zone badges correctly', () => {
    const zones = [
      { zone: 'zone1', label: 'Z1 Recovery' },
      { zone: 'zone2', label: 'Z2 Easy' },
      { zone: 'zone3', label: 'Z3 Aerobic' },
      { zone: 'zone4', label: 'Z4 Threshold' },
      { zone: 'zone5', label: 'Z5 Max' },
    ] as const;

    zones.forEach(({ zone, label }) => {
      const { getByText } = renderWithTheme(
        <HeartRateDisplay
          heartRate={120}
          hrZone={zone}
          maxHeartRate={180}
          showMetrics={true}
        />,
      );

      expect(getByText(label)).toBeTruthy();
    });
  });

  it('renders placeholder when showMetrics is false', () => {
    const { getAllByText, queryByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={145}
        hrZone="zone4"
        maxHeartRate={165}
        showMetrics={false}
      />,
    );

    // Both HR and max show "--" when no metrics
    const placeholders = getAllByText('--');
    expect(placeholders.length).toBeGreaterThanOrEqual(1);
    expect(queryByText('145')).toBeNull();
    expect(queryByText(/Z4 Threshold/)).toBeNull();
  });

  it('renders placeholder when heart rate is null', () => {
    const { getAllByText, queryByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={null}
        hrZone={null}
        maxHeartRate={0}
        showMetrics={true}
      />,
    );

    const placeholders = getAllByText('--');
    expect(placeholders.length).toBeGreaterThanOrEqual(1);
    expect(queryByText(/Z\d/)).toBeNull();
  });

  it('does not show zone badge when heart rate is 0', () => {
    const { queryByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={0}
        hrZone="zone1"
        maxHeartRate={0}
        showMetrics={true}
      />,
    );

    expect(queryByText(/Z1 Recovery/)).toBeNull();
  });

  it('does not show zone badge when heart rate is null', () => {
    const { queryByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={null}
        hrZone={null}
        maxHeartRate={0}
        showMetrics={true}
      />,
    );

    expect(queryByText(/Z\d/)).toBeNull();
  });

  it('shows heart icon when heart rate is visible', () => {
    const { getByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={140}
        hrZone="zone3"
        maxHeartRate={165}
        showMetrics={true}
      />,
    );

    // Heart icon is rendered as Unicode character
    expect(getByText('\u2665')).toBeTruthy();
  });

  it('does not show heart icon when showMetrics is false', () => {
    const { queryByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={140}
        hrZone="zone3"
        maxHeartRate={165}
        showMetrics={false}
      />,
    );

    expect(queryByText('\u2665')).toBeNull();
  });

  it('has correct accessibility label with all data', () => {
    const { getByLabelText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={155}
        hrZone="zone4"
        maxHeartRate={170}
        showMetrics={true}
      />,
    );

    expect(
      getByLabelText(/Heart rate: 155 beats per minute.*Zone: Threshold.*Maximum: 170/),
    ).toBeTruthy();
  });

  it('has correct accessibility label without zone', () => {
    const { getByLabelText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={120}
        hrZone={null}
        maxHeartRate={165}
        showMetrics={true}
      />,
    );

    expect(
      getByLabelText(/Heart rate: 120 beats per minute/),
    ).toBeTruthy();
  });

  it('has correct accessibility label when no metrics', () => {
    const { getByLabelText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={null}
        hrZone={null}
        maxHeartRate={0}
        showMetrics={false}
      />,
    );

    expect(
      getByLabelText(/Heart rate: no data/),
    ).toBeTruthy();
  });

  it('renders with pulse animation disabled', () => {
    const { getByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={145}
        hrZone="zone4"
        maxHeartRate={165}
        showMetrics={true}
        showPulseAnimation={false}
      />,
    );

    // Should still render the heart rate
    expect(getByText('145')).toBeTruthy();
  });

  it('renders with custom style', () => {
    const { getByText } = renderWithTheme(
      <HeartRateDisplay
        heartRate={130}
        hrZone="zone3"
        maxHeartRate={160}
        showMetrics={true}
        style={{ width: 200 }}
      />,
    );

    expect(getByText('130')).toBeTruthy();
  });
});
