/**
 * Secondary Metrics Component Tests
 */

import React from 'react';
import { render } from '@testing-library/react-native';
import {
  SpeedDisplay,
  DistanceDisplay,
  ElapsedTimeDisplay,
  CaloriesDisplay,
} from '../../src/components/SecondaryMetrics';
import { useSettingsStore } from '../../src/stores/settingsStore';

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () => ({
  getItem: jest.fn(() => Promise.resolve(null)),
  setItem: jest.fn(() => Promise.resolve()),
  removeItem: jest.fn(() => Promise.resolve()),
  multiRemove: jest.fn(() => Promise.resolve()),
}));

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

describe('SpeedDisplay', () => {
  beforeEach(() => {
    // Reset store to metric units
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
  });

  it('should render speed in km/h for metric units', () => {
    const { getByText } = render(
      <SpeedDisplay speedKph={32.5} showMetrics={true} />
    );

    expect(getByText('32.5')).toBeTruthy();
    expect(getByText('km/h')).toBeTruthy();
    expect(getByText('SPEED')).toBeTruthy();
  });

  it('should render speed in mph for imperial units', () => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        units: 'imperial',
      },
    });

    const { getByText } = render(
      <SpeedDisplay speedKph={32.5} showMetrics={true} />
    );

    expect(getByText('20.2')).toBeTruthy();
    expect(getByText('mph')).toBeTruthy();
  });

  it('should render placeholder when showMetrics is false', () => {
    const { getByText } = render(
      <SpeedDisplay speedKph={32.5} showMetrics={false} />
    );

    expect(getByText('--')).toBeTruthy();
  });

  it('should override units when explicitly provided', () => {
    const { getByText } = render(
      <SpeedDisplay speedKph={32.5} showMetrics={true} units="imperial" />
    );

    expect(getByText('mph')).toBeTruthy();
  });

  it('should have proper accessibility label', () => {
    const { getByLabelText } = render(
      <SpeedDisplay speedKph={32.5} showMetrics={true} />
    );

    expect(
      getByLabelText('Speed: 32.5 kilometers per hour')
    ).toBeTruthy();
  });
});

describe('DistanceDisplay', () => {
  beforeEach(() => {
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
  });

  it('should render distance in km for metric units', () => {
    const { getByText } = render(
      <DistanceDisplay distanceKm={15.234} showMetrics={true} />
    );

    expect(getByText('15.2')).toBeTruthy();
    expect(getByText('km')).toBeTruthy();
    expect(getByText('DISTANCE')).toBeTruthy();
  });

  it('should render distance in miles for imperial units', () => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        units: 'imperial',
      },
    });

    const { getByText } = render(
      <DistanceDisplay distanceKm={16.09} showMetrics={true} />
    );

    expect(getByText('10.00')).toBeTruthy();
    expect(getByText('mi')).toBeTruthy();
  });

  it('should render small distances with 2 decimals', () => {
    const { getByText } = render(
      <DistanceDisplay distanceKm={5.678} showMetrics={true} />
    );

    expect(getByText('5.68')).toBeTruthy();
  });

  it('should render placeholder when showMetrics is false', () => {
    const { getByText } = render(
      <DistanceDisplay distanceKm={15.234} showMetrics={false} />
    );

    expect(getByText('0.00')).toBeTruthy();
  });

  it('should have proper accessibility label', () => {
    const { getByLabelText } = render(
      <DistanceDisplay distanceKm={15.234} showMetrics={true} />
    );

    expect(
      getByLabelText('Distance: 15.2 kilometers')
    ).toBeTruthy();
  });
});

describe('ElapsedTimeDisplay', () => {
  it('should render time in M:SS format for short durations', () => {
    const { getByText } = render(
      <ElapsedTimeDisplay elapsedSecs={125} showMetrics={true} />
    );

    expect(getByText('2:05')).toBeTruthy();
    expect(getByText('TIME')).toBeTruthy();
  });

  it('should render time in H:MM:SS format for long durations', () => {
    const { getByText } = render(
      <ElapsedTimeDisplay elapsedSecs={3725} showMetrics={true} />
    );

    expect(getByText('1:02:05')).toBeTruthy();
  });

  it('should render zero time correctly', () => {
    const { getByText } = render(
      <ElapsedTimeDisplay elapsedSecs={0} showMetrics={true} />
    );

    expect(getByText('0:00')).toBeTruthy();
  });

  it('should render placeholder when showMetrics is false', () => {
    const { getByText } = render(
      <ElapsedTimeDisplay elapsedSecs={125} showMetrics={false} />
    );

    expect(getByText('0:00')).toBeTruthy();
  });

  it('should have proper accessibility label with readable time', () => {
    const { getByLabelText } = render(
      <ElapsedTimeDisplay elapsedSecs={3725} showMetrics={true} />
    );

    expect(
      getByLabelText('Elapsed time: 1 hour 2 minutes 5 seconds')
    ).toBeTruthy();
  });
});

describe('CaloriesDisplay', () => {
  it('should render calorie count', () => {
    const { getByText } = render(
      <CaloriesDisplay calories={523} showMetrics={true} />
    );

    expect(getByText('523')).toBeTruthy();
    expect(getByText('kcal')).toBeTruthy();
    expect(getByText('CALORIES')).toBeTruthy();
  });

  it('should round decimal calories', () => {
    const { getByText } = render(
      <CaloriesDisplay calories={523.7} showMetrics={true} />
    );

    expect(getByText('524')).toBeTruthy();
  });

  it('should format large calorie values as X.Xk', () => {
    const { getByText } = render(
      <CaloriesDisplay calories={1500} showMetrics={true} />
    );

    expect(getByText('1.5k')).toBeTruthy();
  });

  it('should render placeholder when showMetrics is false', () => {
    const { getByText } = render(
      <CaloriesDisplay calories={523} showMetrics={false} />
    );

    expect(getByText('0')).toBeTruthy();
  });

  it('should have proper accessibility label', () => {
    const { getByLabelText } = render(
      <CaloriesDisplay calories={523} showMetrics={true} />
    );

    expect(
      getByLabelText('Calories: 523 kilocalories burned')
    ).toBeTruthy();
  });
});

describe('Styling and Layout', () => {
  it('should apply custom styles to SpeedDisplay', () => {
    const { toJSON } = render(
      <SpeedDisplay
        speedKph={32.5}
        showMetrics={true}
        style={{ marginTop: 10 }}
      />
    );

    const tree = toJSON();
    expect(tree).toBeTruthy();
  });

  it('should apply custom styles to DistanceDisplay', () => {
    const { toJSON } = render(
      <DistanceDisplay
        distanceKm={15.234}
        showMetrics={true}
        style={{ marginTop: 10 }}
      />
    );

    const tree = toJSON();
    expect(tree).toBeTruthy();
  });

  it('should apply custom styles to ElapsedTimeDisplay', () => {
    const { toJSON } = render(
      <ElapsedTimeDisplay
        elapsedSecs={125}
        showMetrics={true}
        style={{ marginTop: 10 }}
      />
    );

    const tree = toJSON();
    expect(tree).toBeTruthy();
  });

  it('should apply custom styles to CaloriesDisplay', () => {
    const { toJSON } = render(
      <CaloriesDisplay
        calories={523}
        showMetrics={true}
        style={{ marginTop: 10 }}
      />
    );

    const tree = toJSON();
    expect(tree).toBeTruthy();
  });
});
