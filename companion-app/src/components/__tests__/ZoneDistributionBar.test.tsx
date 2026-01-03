/**
 * ZoneDistributionBar Component Tests
 */

import React from 'react';
import { render, screen } from '@testing-library/react-native';
import {
  ZoneDistributionBar,
  getPowerZoneData,
  getHrZoneData,
  ZoneData,
} from '../ZoneDistributionBar';

describe('ZoneDistributionBar', () => {
  const mockPowerZones: ZoneData[] = [
    { zone: 'z1', label: 'Z1 Recovery', shortLabel: 'Z1', seconds: 600, color: '#808080' },
    { zone: 'z2', label: 'Z2 Endurance', shortLabel: 'Z2', seconds: 1200, color: '#0080FF' },
    { zone: 'z3', label: 'Z3 Tempo', shortLabel: 'Z3', seconds: 900, color: '#00C864' },
    { zone: 'z4', label: 'Z4 Threshold', shortLabel: 'Z4', seconds: 300, color: '#FFC800' },
    { zone: 'z5', label: 'Z5 VO2max', shortLabel: 'Z5', seconds: 0, color: '#FF8000' },
    { zone: 'z6', label: 'Z6 Anaerobic', shortLabel: 'Z6', seconds: 0, color: '#FF3232' },
    { zone: 'z7', label: 'Z7 Neuromuscular', shortLabel: 'Z7', seconds: 0, color: '#B400B4' },
  ];

  it('renders with title', () => {
    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={mockPowerZones}
      />
    );

    expect(screen.getByText('Power Zone Distribution')).toBeTruthy();
  });

  it('shows total time', () => {
    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={mockPowerZones}
      />
    );

    // Total: 600 + 1200 + 900 + 300 = 3000 seconds = 50:00
    expect(screen.getByText('Total: 50:00')).toBeTruthy();
  });

  it('shows legend by default', () => {
    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={mockPowerZones}
        showLegend
      />
    );

    // Check that zone labels are shown (may appear multiple times in bar and legend)
    const z1Elements = screen.getAllByText('Z1');
    const z2Elements = screen.getAllByText('Z2');
    expect(z1Elements.length).toBeGreaterThan(0);
    expect(z2Elements.length).toBeGreaterThan(0);
  });

  it('hides legend when showLegend is false', () => {
    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={mockPowerZones}
        showLegend={false}
      />
    );

    // Zone labels should not be visible
    expect(screen.queryByText('10:00')).toBeNull();
  });

  it('displays zone times in legend', () => {
    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={mockPowerZones}
        showLegend
      />
    );

    // Z1 = 600s = 10:00
    expect(screen.getByText('10:00')).toBeTruthy();
    // Z2 = 1200s = 20:00
    expect(screen.getByText('20:00')).toBeTruthy();
  });

  it('shows no data message when all zones are zero', () => {
    const emptyZones: ZoneData[] = mockPowerZones.map(z => ({ ...z, seconds: 0 }));

    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={emptyZones}
      />
    );

    expect(screen.getByText('No zone data available')).toBeTruthy();
  });

  it('has correct accessibility label', () => {
    render(
      <ZoneDistributionBar
        title="Power Zone Distribution"
        zones={mockPowerZones}
      />
    );

    // The component should have an accessibility label
    const container = screen.getByRole('summary');
    expect(container.props.accessibilityLabel).toContain('Power Zone Distribution');
  });
});

describe('getPowerZoneData', () => {
  it('returns zone data for power distribution', () => {
    const distribution = {
      z1_recovery: 600,
      z2_endurance: 1200,
      z3_tempo: 900,
      z4_threshold: 300,
      z5_vo2max: 0,
      z6_anaerobic: 0,
      z7_neuromuscular: 0,
    };

    const result = getPowerZoneData(distribution);

    expect(result.length).toBe(7);
    expect(result[0]).toEqual(expect.objectContaining({
      zone: 'z1',
      shortLabel: 'Z1',
      seconds: 600,
    }));
    expect(result[1].seconds).toBe(1200);
    expect(result[2].seconds).toBe(900);
  });

  it('handles null distribution', () => {
    const result = getPowerZoneData(null);

    expect(result.length).toBe(7);
    result.forEach(zone => {
      expect(zone.seconds).toBe(0);
    });
  });

  it('handles undefined distribution', () => {
    const result = getPowerZoneData(undefined);

    expect(result.length).toBe(7);
    result.forEach(zone => {
      expect(zone.seconds).toBe(0);
    });
  });
});

describe('getHrZoneData', () => {
  it('returns zone data for HR distribution', () => {
    const distribution = {
      z1: 300,
      z2: 600,
      z3: 1200,
      z4: 500,
      z5: 100,
    };

    const result = getHrZoneData(distribution);

    expect(result.length).toBe(5);
    expect(result[0]).toEqual(expect.objectContaining({
      zone: 'z1',
      shortLabel: 'Z1',
      seconds: 300,
    }));
    expect(result[2].label).toBe('Z3 Aerobic');
    expect(result[4].seconds).toBe(100);
  });

  it('handles null distribution', () => {
    const result = getHrZoneData(null);

    expect(result.length).toBe(5);
    result.forEach(zone => {
      expect(zone.seconds).toBe(0);
    });
  });

  it('handles undefined distribution', () => {
    const result = getHrZoneData(undefined);

    expect(result.length).toBe(5);
    result.forEach(zone => {
      expect(zone.seconds).toBe(0);
    });
  });
});
