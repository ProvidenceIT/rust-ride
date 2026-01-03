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
