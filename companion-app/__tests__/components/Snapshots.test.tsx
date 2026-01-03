/**
 * UI Component Snapshot Tests
 *
 * Snapshot tests for visual regression testing of UI components.
 * These tests capture the rendered output and compare against stored snapshots.
 */

import React from 'react';
import { create, ReactTestRenderer, act } from 'react-test-renderer';
import { Text } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { ThemeProvider } from '../../src/theme';
import {
  MetricCard,
  Button,
  IconButton,
  ConnectionStatus,
  LoadingSpinner,
  FullScreenLoader,
  PowerDisplay,
  HeartRateDisplay,
  CadenceDisplay,
  NoSessionState,
  Toast,
} from '../../src/components';

// Mock Icon component for consistent snapshots
const MockIcon = ({ size, color }: { size?: number; color?: string }) => (
  <Text testID="mock-icon">{`Icon:${size}x${color}`}</Text>
);

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

// Helper to render with theme
const renderWithTheme = (component: React.ReactElement): ReactTestRenderer => {
  let tree: ReactTestRenderer;
  act(() => {
    tree = create(<ThemeProvider>{component}</ThemeProvider>);
  });
  return tree!;
};

// Helper to render with theme and SafeArea (for components using useSafeAreaInsets)
const renderWithThemeAndSafeArea = (component: React.ReactElement): ReactTestRenderer => {
  let tree: ReactTestRenderer;
  act(() => {
    tree = create(
      <SafeAreaProvider
        initialMetrics={{
          frame: { x: 0, y: 0, width: 375, height: 812 },
          insets: { top: 47, left: 0, right: 0, bottom: 34 },
        }}
      >
        <ThemeProvider>{component}</ThemeProvider>
      </SafeAreaProvider>,
    );
  });
  return tree!;
};

describe('MetricCard Snapshots', () => {
  it('renders basic metric card correctly', () => {
    const tree = renderWithTheme(
      <MetricCard value={250} unit="W" label="Power" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders small size correctly', () => {
    const tree = renderWithTheme(
      <MetricCard value={90} unit="rpm" label="Cadence" size="small" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders large size correctly', () => {
    const tree = renderWithTheme(
      <MetricCard value={285} unit="W" label="Power" size="large" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders with accent color correctly', () => {
    const tree = renderWithTheme(
      <MetricCard
        value={300}
        unit="W"
        label="Power"
        accentColor="#FFC107"
        showAccentBorder
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders with secondary value correctly', () => {
    const tree = renderWithTheme(
      <MetricCard
        value={280}
        unit="W"
        label="Power"
        secondaryValue={275}
        secondaryLabel="3s avg"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders with target value correctly', () => {
    const tree = renderWithTheme(
      <MetricCard
        value={250}
        unit="W"
        label="Power"
        targetValue={275}
        targetLabel="Target"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders placeholder value correctly', () => {
    const tree = renderWithTheme(
      <MetricCard value="--" unit="W" label="Power" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('Button Snapshots', () => {
  it('renders primary button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Start Workout" variant="primary" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders secondary button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Cancel" variant="secondary" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders outline button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Learn More" variant="outline" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders danger button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Stop Session" variant="danger" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders ghost button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Skip" variant="ghost" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders small button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Small" size="small" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders large button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Large Action" size="large" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders full width button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Full Width" fullWidth />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders disabled button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Disabled" disabled />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders loading button correctly', () => {
    const tree = renderWithTheme(
      <Button title="Loading" loading />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('IconButton Snapshots', () => {
  it('renders default icon button correctly', () => {
    const tree = renderWithTheme(
      <IconButton
        icon={<MockIcon size={24} color="#fff" />}
        accessibilityLabel="Play"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders primary icon button correctly', () => {
    const tree = renderWithTheme(
      <IconButton
        icon={<MockIcon size={24} color="#fff" />}
        variant="primary"
        accessibilityLabel="Play"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders danger icon button correctly', () => {
    const tree = renderWithTheme(
      <IconButton
        icon={<MockIcon size={24} color="#fff" />}
        variant="danger"
        accessibilityLabel="Stop"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders circular icon button correctly', () => {
    const tree = renderWithTheme(
      <IconButton
        icon={<MockIcon size={24} color="#fff" />}
        circular
        accessibilityLabel="Play"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders loading icon button correctly', () => {
    const tree = renderWithTheme(
      <IconButton
        icon={<MockIcon size={24} color="#fff" />}
        loading
        accessibilityLabel="Loading"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('ConnectionStatus Snapshots', () => {
  it('renders dot variant correctly', () => {
    const tree = renderWithTheme(
      <ConnectionStatus status="connected" variant="dot" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders badge variant correctly', () => {
    const tree = renderWithTheme(
      <ConnectionStatus status="connected" variant="badge" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders full variant correctly', () => {
    const tree = renderWithTheme(
      <ConnectionStatus
        status="connected"
        variant="full"
        serverName="RustRide-PC:9876"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders disconnected state correctly', () => {
    const tree = renderWithTheme(
      <ConnectionStatus status="disconnected" variant="badge" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders connecting state correctly', () => {
    const tree = renderWithTheme(
      <ConnectionStatus status="connecting" variant="badge" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders authenticated state correctly', () => {
    const tree = renderWithTheme(
      <ConnectionStatus status="authenticated" variant="badge" />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('LoadingSpinner Snapshots', () => {
  it('renders small spinner correctly', () => {
    const tree = renderWithTheme(<LoadingSpinner size="small" />);
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders medium spinner correctly', () => {
    const tree = renderWithTheme(<LoadingSpinner size="medium" />);
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders large spinner correctly', () => {
    const tree = renderWithTheme(<LoadingSpinner size="large" />);
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders with message correctly', () => {
    const tree = renderWithTheme(
      <LoadingSpinner message="Loading data..." />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('FullScreenLoader Snapshots', () => {
  it('renders full screen loader correctly', () => {
    const tree = renderWithTheme(
      <FullScreenLoader message="Please wait..." />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('PowerDisplay Snapshots', () => {
  it('renders with power data correctly', () => {
    const tree = renderWithTheme(
      <PowerDisplay
        power={285}
        power3sAvg={280}
        powerZone="threshold"
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders with target power correctly', () => {
    const tree = renderWithTheme(
      <PowerDisplay
        power={275}
        power3sAvg={272}
        powerZone="threshold"
        targetPower={280}
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders recovery zone correctly', () => {
    const tree = renderWithTheme(
      <PowerDisplay
        power={100}
        power3sAvg={95}
        powerZone="recovery"
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders vo2max zone correctly', () => {
    const tree = renderWithTheme(
      <PowerDisplay
        power={350}
        power3sAvg={345}
        powerZone="vo2max"
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders without metrics correctly', () => {
    const tree = renderWithTheme(
      <PowerDisplay
        power={0}
        power3sAvg={0}
        powerZone="recovery"
        showMetrics={false}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('HeartRateDisplay Snapshots', () => {
  it('renders with HR data correctly', () => {
    const tree = renderWithTheme(
      <HeartRateDisplay
        heartRate={155}
        hrZone="zone4"
        maxHeartRate={172}
        showMetrics={true}
        showPulseAnimation={false}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders zone1 correctly', () => {
    const tree = renderWithTheme(
      <HeartRateDisplay
        heartRate={95}
        hrZone="zone1"
        maxHeartRate={172}
        showMetrics={true}
        showPulseAnimation={false}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders zone5 correctly', () => {
    const tree = renderWithTheme(
      <HeartRateDisplay
        heartRate={175}
        hrZone="zone5"
        maxHeartRate={180}
        showMetrics={true}
        showPulseAnimation={false}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders without metrics correctly', () => {
    const tree = renderWithTheme(
      <HeartRateDisplay
        heartRate={null}
        hrZone={null}
        maxHeartRate={0}
        showMetrics={false}
        showPulseAnimation={false}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('CadenceDisplay Snapshots', () => {
  it('renders with cadence data correctly', () => {
    const tree = renderWithTheme(
      <CadenceDisplay
        cadence={92}
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders with target cadence correctly', () => {
    const tree = renderWithTheme(
      <CadenceDisplay
        cadence={88}
        targetCadence={90}
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders high cadence warning correctly', () => {
    const tree = renderWithTheme(
      <CadenceDisplay
        cadence={105}
        targetCadence={90}
        tolerance={5}
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders low cadence warning correctly', () => {
    const tree = renderWithTheme(
      <CadenceDisplay
        cadence={75}
        targetCadence={90}
        tolerance={5}
        showMetrics={true}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders without metrics correctly', () => {
    const tree = renderWithTheme(
      <CadenceDisplay
        cadence={null}
        showMetrics={false}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('NoSessionState Snapshots', () => {
  it('renders disconnected state correctly', () => {
    const tree = renderWithTheme(
      <NoSessionState
        connectionStatus="disconnected"
        onConnectPress={() => {}}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders connecting state correctly', () => {
    const tree = renderWithTheme(
      <NoSessionState
        connectionStatus="connecting"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders connected state correctly', () => {
    const tree = renderWithTheme(
      <NoSessionState
        connectionStatus="authenticated"
        serverName="RustRide-PC"
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});

describe('Toast Snapshots', () => {
  it('renders success toast correctly', () => {
    const tree = renderWithThemeAndSafeArea(
      <Toast
        toast={{
          id: '1',
          variant: 'success',
          message: 'Workout saved successfully',
        }}
        onDismiss={() => {}}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders error toast correctly', () => {
    const tree = renderWithThemeAndSafeArea(
      <Toast
        toast={{
          id: '2',
          variant: 'error',
          message: 'Connection failed',
        }}
        onDismiss={() => {}}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders warning toast correctly', () => {
    const tree = renderWithThemeAndSafeArea(
      <Toast
        toast={{
          id: '3',
          variant: 'warning',
          message: 'Low battery',
        }}
        onDismiss={() => {}}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders info toast correctly', () => {
    const tree = renderWithThemeAndSafeArea(
      <Toast
        toast={{
          id: '4',
          variant: 'info',
          message: 'New update available',
        }}
        onDismiss={() => {}}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });

  it('renders toast with action correctly', () => {
    const tree = renderWithThemeAndSafeArea(
      <Toast
        toast={{
          id: '5',
          variant: 'success',
          message: 'Great job! You completed your FTP workout.',
          action: {
            label: 'View Details',
            onPress: () => {},
          },
        }}
        onDismiss={() => {}}
      />,
    );
    expect(tree.toJSON()).toMatchSnapshot();
  });
});
