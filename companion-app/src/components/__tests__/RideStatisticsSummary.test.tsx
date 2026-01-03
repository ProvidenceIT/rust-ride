/**
 * RideStatisticsSummary Component Tests
 */

import React from 'react';
import { render, screen } from '@testing-library/react-native';
import { RideStatisticsSummary } from '../RideStatisticsSummary';

describe('RideStatisticsSummary', () => {
  it('renders TSS value', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={0.92}
        calories={650}
      />
    );

    expect(screen.getByText('85')).toBeTruthy();
    expect(screen.getByText('TSS')).toBeTruthy();
  });

  it('renders intensity factor value', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={0.92}
        calories={650}
      />
    );

    expect(screen.getByText('0.92')).toBeTruthy();
    expect(screen.getByText('Intensity Factor')).toBeTruthy();
  });

  it('renders calories value', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={0.92}
        calories={650}
      />
    );

    expect(screen.getByText('650')).toBeTruthy();
    expect(screen.getByText('Calories')).toBeTruthy();
  });

  it('shows TSS intensity level - Easy', () => {
    render(
      <RideStatisticsSummary
        tss={45}
        intensityFactor={0.70}
        calories={400}
      />
    );

    expect(screen.getByText('Easy')).toBeTruthy();
  });

  it('shows TSS intensity level - Moderate', () => {
    render(
      <RideStatisticsSummary
        tss={75}
        intensityFactor={0.85}
        calories={550}
      />
    );

    expect(screen.getByText('Moderate')).toBeTruthy();
  });

  it('shows TSS intensity level - Hard', () => {
    render(
      <RideStatisticsSummary
        tss={125}
        intensityFactor={0.95}
        calories={800}
      />
    );

    expect(screen.getByText('Hard')).toBeTruthy();
  });

  it('shows TSS intensity level - Very Hard', () => {
    render(
      <RideStatisticsSummary
        tss={175}
        intensityFactor={1.0}
        calories={1000}
      />
    );

    expect(screen.getByText('Very Hard')).toBeTruthy();
  });

  it('shows TSS intensity level - Epic', () => {
    render(
      <RideStatisticsSummary
        tss={220}
        intensityFactor={1.1}
        calories={1200}
      />
    );

    expect(screen.getByText('Epic')).toBeTruthy();
  });

  it('shows IF description - Endurance', () => {
    render(
      <RideStatisticsSummary
        tss={50}
        intensityFactor={0.65}
        calories={400}
      />
    );

    expect(screen.getByText('Endurance')).toBeTruthy();
  });

  it('shows IF description - Tempo', () => {
    render(
      <RideStatisticsSummary
        tss={75}
        intensityFactor={0.82}
        calories={550}
      />
    );

    expect(screen.getByText('Tempo')).toBeTruthy();
  });

  it('shows IF description - Threshold', () => {
    render(
      <RideStatisticsSummary
        tss={100}
        intensityFactor={0.95}
        calories={700}
      />
    );

    expect(screen.getByText('Threshold')).toBeTruthy();
  });

  it('handles null TSS', () => {
    render(
      <RideStatisticsSummary
        tss={null}
        intensityFactor={0.92}
        calories={650}
      />
    );

    // Should show '--' for missing TSS
    const dashElements = screen.getAllByText('--');
    expect(dashElements.length).toBeGreaterThan(0);
  });

  it('handles null intensity factor', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={null}
        calories={650}
      />
    );

    // Should show '--' for missing IF
    const dashElements = screen.getAllByText('--');
    expect(dashElements.length).toBeGreaterThan(0);
  });

  it('handles zero calories', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={0.92}
        calories={0}
      />
    );

    // Should show '--' for zero calories
    const dashElements = screen.getAllByText('--');
    expect(dashElements.length).toBeGreaterThan(0);
  });

  it('has correct accessibility label', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={0.92}
        calories={650}
      />
    );

    const container = screen.getByRole('summary');
    expect(container.props.accessibilityLabel).toContain('Training Summary');
    expect(container.props.accessibilityLabel).toContain('TSS');
    expect(container.props.accessibilityLabel).toContain('Intensity Factor');
    expect(container.props.accessibilityLabel).toContain('calories');
  });

  it('rounds TSS to whole number', () => {
    render(
      <RideStatisticsSummary
        tss={85.7}
        intensityFactor={0.92}
        calories={650}
      />
    );

    expect(screen.getByText('86')).toBeTruthy();
  });

  it('rounds calories to whole number', () => {
    render(
      <RideStatisticsSummary
        tss={85}
        intensityFactor={0.92}
        calories={649.8}
      />
    );

    expect(screen.getByText('650')).toBeTruthy();
  });
});
