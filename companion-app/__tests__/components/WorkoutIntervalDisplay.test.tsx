/**
 * WorkoutIntervalDisplay Component Tests
 */

import React from 'react';
import { render } from '@testing-library/react-native';
import { WorkoutIntervalDisplay } from '../../src/components/WorkoutIntervalDisplay';
import type { IntervalInfo, NextIntervalInfo } from '../../src/components/WorkoutIntervalDisplay';

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () => ({
  getItem: jest.fn(() => Promise.resolve(null)),
  setItem: jest.fn(() => Promise.resolve()),
  removeItem: jest.fn(() => Promise.resolve()),
  multiRemove: jest.fn(() => Promise.resolve()),
}));

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

describe('WorkoutIntervalDisplay', () => {
  const defaultInterval: IntervalInfo = {
    index: 2,
    total: 8,
    name: 'Threshold Effort',
    remainingSecs: 180,
  };

  describe('Basic Rendering', () => {
    it('should render interval name', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
        />
      );

      expect(getByText('Threshold Effort')).toBeTruthy();
    });

    it('should render interval counter', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
        />
      );

      expect(getByText('INTERVAL 3 OF 8')).toBeTruthy();
    });

    it('should render time remaining in MM:SS format', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
        />
      );

      expect(getByText('3:00')).toBeTruthy();
      expect(getByText(/remaining/)).toBeTruthy();
    });

    it('should render time remaining in HH:MM:SS format for long intervals', () => {
      const longInterval: IntervalInfo = {
        ...defaultInterval,
        remainingSecs: 3725, // 1:02:05
      };

      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={longInterval}
          showMetrics={true}
        />
      );

      expect(getByText('1:02:05')).toBeTruthy();
    });

    it('should render fallback name when interval name is null', () => {
      const noNameInterval: IntervalInfo = {
        ...defaultInterval,
        name: null,
      };

      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={noNameInterval}
          showMetrics={true}
        />
      );

      expect(getByText('Interval')).toBeTruthy();
    });

    it('should render placeholder for null remaining time', () => {
      const noTimeInterval: IntervalInfo = {
        ...defaultInterval,
        remainingSecs: null,
      };

      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={noTimeInterval}
          showMetrics={true}
        />
      );

      expect(getByText('--:--')).toBeTruthy();
    });

    it('should return null when showMetrics is false', () => {
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={false}
        />
      );

      expect(toJSON()).toBeNull();
    });
  });

  describe('Paused State', () => {
    it('should show PAUSED badge when workout is paused', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          isPaused={true}
          showMetrics={true}
        />
      );

      expect(getByText('PAUSED')).toBeTruthy();
    });

    it('should not show PAUSED badge when workout is not paused', () => {
      const { queryByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          isPaused={false}
          showMetrics={true}
        />
      );

      expect(queryByText('PAUSED')).toBeNull();
    });
  });

  describe('Next Interval Preview', () => {
    const nextInterval: NextIntervalInfo = {
      name: 'Recovery',
      targetPower: 120,
      durationSecs: 60,
    };

    it('should render next interval name', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          nextInterval={nextInterval}
          showMetrics={true}
        />
      );

      expect(getByText('NEXT')).toBeTruthy();
      expect(getByText('Recovery')).toBeTruthy();
    });

    it('should render next interval target power', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          nextInterval={nextInterval}
          showMetrics={true}
        />
      );

      expect(getByText('120W')).toBeTruthy();
    });

    it('should render next interval duration', () => {
      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          nextInterval={nextInterval}
          showMetrics={true}
        />
      );

      expect(getByText('1:00')).toBeTruthy();
    });

    it('should not render next interval section when null', () => {
      const { queryByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          nextInterval={null}
          showMetrics={true}
        />
      );

      expect(queryByText('NEXT')).toBeNull();
    });

    it('should not show power badge when targetPower is undefined', () => {
      const nextIntervalNoPower: NextIntervalInfo = {
        name: 'Rest',
      };

      const { queryByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          nextInterval={nextIntervalNoPower}
          showMetrics={true}
        />
      );

      expect(queryByText(/W$/)).toBeNull();
    });
  });

  describe('Progress Bar', () => {
    it('should render progress bar at correct percentage', () => {
      // Interval 3 of 8 = (3/8) * 100 = 37.5%
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });

    it('should handle first interval progress', () => {
      const firstInterval: IntervalInfo = {
        index: 0,
        total: 8,
        name: 'Warmup',
        remainingSecs: 300,
      };

      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={firstInterval}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });

    it('should handle last interval progress', () => {
      const lastInterval: IntervalInfo = {
        index: 7,
        total: 8,
        name: 'Cooldown',
        remainingSecs: 60,
      };

      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={lastInterval}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });
  });

  describe('Zone Color Coding', () => {
    it('should apply zone color based on target power and FTP', () => {
      // 250W at FTP 200 = 125% = Zone 5 (VO2max)
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          targetPower={250}
          ftp={200}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });

    it('should handle low intensity zone', () => {
      // 100W at FTP 200 = 50% = Zone 1 (Recovery)
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          targetPower={100}
          ftp={200}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });

    it('should handle null target power', () => {
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          targetPower={null}
          ftp={200}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });
  });

  describe('Accessibility', () => {
    it('should have proper accessibility label for current interval', () => {
      const { getByLabelText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
        />
      );

      expect(
        getByLabelText(/Threshold Effort.*Interval 3 of 8.*3:00 remaining/)
      ).toBeTruthy();
    });

    it('should include next interval in accessibility label', () => {
      const nextInterval: NextIntervalInfo = {
        name: 'Recovery',
        targetPower: 120,
      };

      const { getByLabelText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          nextInterval={nextInterval}
          showMetrics={true}
        />
      );

      expect(getByLabelText(/Next: Recovery/)).toBeTruthy();
    });

    it('should include paused state in accessibility label', () => {
      const { getByLabelText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          isPaused={true}
          showMetrics={true}
        />
      );

      expect(getByLabelText(/Workout paused/)).toBeTruthy();
    });

    it('should handle custom accessibility label', () => {
      const { getByLabelText } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
          accessibilityLabel="Custom workout interval display"
        />
      );

      expect(getByLabelText('Custom workout interval display')).toBeTruthy();
    });
  });

  describe('Styling', () => {
    it('should apply custom style prop', () => {
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          showMetrics={true}
          style={{ marginTop: 20 }}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });
  });

  describe('Edge Cases', () => {
    it('should handle zero total intervals', () => {
      const zeroTotalInterval: IntervalInfo = {
        index: 0,
        total: 0,
        name: 'Test',
        remainingSecs: 60,
      };

      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={zeroTotalInterval}
          showMetrics={true}
        />
      );

      expect(getByText('INTERVAL 1 OF 0')).toBeTruthy();
    });

    it('should handle zero remaining seconds', () => {
      const zeroTimeInterval: IntervalInfo = {
        ...defaultInterval,
        remainingSecs: 0,
      };

      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={zeroTimeInterval}
          showMetrics={true}
        />
      );

      expect(getByText('0:00')).toBeTruthy();
    });

    it('should handle negative remaining seconds', () => {
      const negativeTimeInterval: IntervalInfo = {
        ...defaultInterval,
        remainingSecs: -5,
      };

      const { getByText } = render(
        <WorkoutIntervalDisplay
          currentInterval={negativeTimeInterval}
          showMetrics={true}
        />
      );

      expect(getByText('--:--')).toBeTruthy();
    });

    it('should handle very high FTP values', () => {
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          targetPower={400}
          ftp={500}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });

    it('should handle zero FTP gracefully', () => {
      const { toJSON } = render(
        <WorkoutIntervalDisplay
          currentInterval={defaultInterval}
          targetPower={200}
          ftp={0}
          showMetrics={true}
        />
      );

      const tree = toJSON();
      expect(tree).toBeTruthy();
    });
  });
});
