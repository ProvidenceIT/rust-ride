/**
 * DateRangePickerModal Component
 *
 * Modal for selecting a custom date range for filtering ride history.
 * Provides quick presets and manual date input.
 */

import React, { useState, useCallback, useEffect } from 'react';
import {
  View,
  Text,
  Modal,
  StyleSheet,
  Pressable,
  TextInput,
  KeyboardAvoidingView,
  Platform,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';
import { Button } from './Button';

/**
 * DateRangePickerModal props
 */
export interface DateRangePickerModalProps {
  /** Whether the modal is visible */
  visible: boolean;
  /** Called when the modal should close */
  onClose: () => void;
  /** Called when a date range is selected */
  onSelect: (startDate: string, endDate: string) => void;
  /** Initial start date (ISO string) */
  initialStartDate?: string | null;
  /** Initial end date (ISO string) */
  initialEndDate?: string | null;
}

/**
 * Quick preset options for common date ranges
 */
const PRESETS = [
  { label: 'Last 7 Days', days: 7 },
  { label: 'Last 14 Days', days: 14 },
  { label: 'Last 30 Days', days: 30 },
  { label: 'Last 60 Days', days: 60 },
  { label: 'Last 90 Days', days: 90 },
] as const;

/**
 * Parse a date string (YYYY-MM-DD format) to a Date object
 */
function parseDate(dateString: string): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(dateString)) {
    return null;
  }

  const [year, month, day] = dateString.split('-').map(Number);
  const date = new Date(year, month - 1, day);

  // Validate the date is valid
  if (
    date.getFullYear() !== year ||
    date.getMonth() !== month - 1 ||
    date.getDate() !== day
  ) {
    return null;
  }

  return date;
}

/**
 * Format a Date object to YYYY-MM-DD string
 */
function formatDateToInput(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/**
 * Format a Date object to ISO string (for storage)
 */
function formatDateToIso(date: Date): string {
  return date.toISOString();
}

/**
 * Get date from N days ago
 */
function getDateDaysAgo(days: number): Date {
  const date = new Date();
  date.setDate(date.getDate() - days);
  date.setHours(0, 0, 0, 0);
  return date;
}

/**
 * DateRangePickerModal Component
 *
 * Provides a modal for selecting a custom date range.
 * Includes quick presets and manual date input.
 *
 * @example
 * ```tsx
 * <DateRangePickerModal
 *   visible={showPicker}
 *   onClose={() => setShowPicker(false)}
 *   onSelect={(start, end) => handleDateRange(start, end)}
 * />
 * ```
 */
export function DateRangePickerModal({
  visible,
  onClose,
  onSelect,
  initialStartDate,
  initialEndDate,
}: DateRangePickerModalProps): React.JSX.Element {
  const { colors, spacing, borderRadius, typography } = useTheme();
  const { textStyles } = typography;

  // Form state
  const [startDateInput, setStartDateInput] = useState('');
  const [endDateInput, setEndDateInput] = useState('');
  const [startError, setStartError] = useState<string | null>(null);
  const [endError, setEndError] = useState<string | null>(null);

  // Reset form when modal opens
  useEffect(() => {
    if (visible) {
      if (initialStartDate) {
        const date = new Date(initialStartDate);
        setStartDateInput(formatDateToInput(date));
      } else {
        setStartDateInput('');
      }

      if (initialEndDate) {
        const date = new Date(initialEndDate);
        setEndDateInput(formatDateToInput(date));
      } else {
        setEndDateInput('');
      }

      setStartError(null);
      setEndError(null);
    }
  }, [visible, initialStartDate, initialEndDate]);

  /**
   * Handle preset selection
   */
  const handlePresetSelect = useCallback(
    (days: number) => {
      const startDate = getDateDaysAgo(days);
      const endDate = new Date();
      endDate.setHours(23, 59, 59, 999);

      onSelect(formatDateToIso(startDate), formatDateToIso(endDate));
      onClose();
    },
    [onSelect, onClose]
  );

  /**
   * Validate and apply custom date range
   */
  const handleApply = useCallback(() => {
    let hasError = false;

    // Validate start date
    const startDate = parseDate(startDateInput);
    if (!startDateInput.trim()) {
      setStartError('Start date is required');
      hasError = true;
    } else if (!startDate) {
      setStartError('Invalid date format (YYYY-MM-DD)');
      hasError = true;
    } else {
      setStartError(null);
    }

    // Validate end date
    const endDate = parseDate(endDateInput);
    if (!endDateInput.trim()) {
      setEndError('End date is required');
      hasError = true;
    } else if (!endDate) {
      setEndError('Invalid date format (YYYY-MM-DD)');
      hasError = true;
    } else {
      setEndError(null);
    }

    if (hasError || !startDate || !endDate) {
      return;
    }

    // Validate date range
    if (startDate > endDate) {
      setStartError('Start date must be before end date');
      return;
    }

    // Future date check
    const today = new Date();
    today.setHours(23, 59, 59, 999);
    if (endDate > today) {
      setEndError('End date cannot be in the future');
      return;
    }

    // Set end date to end of day for inclusive range
    endDate.setHours(23, 59, 59, 999);
    startDate.setHours(0, 0, 0, 0);

    onSelect(formatDateToIso(startDate), formatDateToIso(endDate));
    onClose();
  }, [startDateInput, endDateInput, onSelect, onClose]);

  /**
   * Handle start date input change
   */
  const handleStartChange = useCallback((text: string) => {
    // Auto-format with dashes
    const cleaned = text.replace(/[^0-9-]/g, '');
    setStartDateInput(cleaned);
    if (startError) {
      setStartError(null);
    }
  }, [startError]);

  /**
   * Handle end date input change
   */
  const handleEndChange = useCallback((text: string) => {
    const cleaned = text.replace(/[^0-9-]/g, '');
    setEndDateInput(cleaned);
    if (endError) {
      setEndError(null);
    }
  }, [endError]);

  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      onRequestClose={onClose}
      statusBarTranslucent
    >
      <KeyboardAvoidingView
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
        style={styles.overlay}
      >
        <Pressable style={styles.backdrop} onPress={onClose}>
          <View />
        </Pressable>

        <View
          style={[
            styles.modal,
            {
              backgroundColor: colors.background,
              borderRadius: borderRadius.lg,
              padding: spacing.lg,
            },
          ]}
        >
          {/* Header */}
          <View style={styles.header}>
            <Text
              style={[styles.title, textStyles.sectionTitle, { color: colors.textPrimary }]}
            >
              Select Date Range
            </Text>
            <Pressable
              onPress={onClose}
              style={styles.closeButton}
              accessibilityRole="button"
              accessibilityLabel="Close date picker"
            >
              <Icon name="close" size={24} color={colors.textSecondary} />
            </Pressable>
          </View>

          {/* Quick presets */}
          <Text
            style={[
              styles.sectionLabel,
              textStyles.label,
              { color: colors.textSecondary, marginTop: spacing.md },
            ]}
          >
            Quick Select
          </Text>
          <View style={[styles.presetsContainer, { marginTop: spacing.xs }]}>
            {PRESETS.map(preset => (
              <Pressable
                key={preset.days}
                style={[
                  styles.presetButton,
                  {
                    backgroundColor: colors.surface,
                    borderColor: colors.border,
                    borderRadius: borderRadius.sm,
                    paddingHorizontal: spacing.sm,
                    paddingVertical: spacing.xs,
                  },
                ]}
                onPress={() => handlePresetSelect(preset.days)}
                accessibilityRole="button"
                accessibilityLabel={preset.label}
                testID={`preset-${preset.days}`}
              >
                <Text style={[styles.presetText, { color: colors.textPrimary }]}>
                  {preset.label}
                </Text>
              </Pressable>
            ))}
          </View>

          {/* Divider */}
          <View
            style={[
              styles.divider,
              { backgroundColor: colors.border, marginVertical: spacing.md },
            ]}
          />

          {/* Custom date input */}
          <Text
            style={[styles.sectionLabel, textStyles.label, { color: colors.textSecondary }]}
          >
            Custom Range
          </Text>

          <View style={styles.dateInputRow}>
            {/* Start date */}
            <View style={[styles.dateInputGroup, { flex: 1 }]}>
              <Text
                style={[
                  styles.inputLabel,
                  textStyles.caption,
                  { color: colors.textSecondary },
                ]}
              >
                Start Date
              </Text>
              <TextInput
                style={[
                  styles.dateInput,
                  {
                    backgroundColor: colors.surface,
                    borderColor: startError ? colors.error : colors.border,
                    borderRadius: borderRadius.sm,
                    color: colors.textPrimary,
                    paddingHorizontal: spacing.sm,
                    paddingVertical: spacing.xs,
                  },
                ]}
                value={startDateInput}
                onChangeText={handleStartChange}
                placeholder="YYYY-MM-DD"
                placeholderTextColor={colors.textMuted}
                keyboardType="numbers-and-punctuation"
                maxLength={10}
                accessibilityLabel="Start date"
                testID="start-date-input"
              />
              {startError && (
                <Text
                  style={[styles.errorText, textStyles.caption, { color: colors.error }]}
                >
                  {startError}
                </Text>
              )}
            </View>

            {/* Arrow */}
            <Icon
              name="arrow-forward"
              size={16}
              color={colors.textSecondary}
              style={[styles.dateArrow, { marginHorizontal: spacing.sm }]}
            />

            {/* End date */}
            <View style={[styles.dateInputGroup, { flex: 1 }]}>
              <Text
                style={[
                  styles.inputLabel,
                  textStyles.caption,
                  { color: colors.textSecondary },
                ]}
              >
                End Date
              </Text>
              <TextInput
                style={[
                  styles.dateInput,
                  {
                    backgroundColor: colors.surface,
                    borderColor: endError ? colors.error : colors.border,
                    borderRadius: borderRadius.sm,
                    color: colors.textPrimary,
                    paddingHorizontal: spacing.sm,
                    paddingVertical: spacing.xs,
                  },
                ]}
                value={endDateInput}
                onChangeText={handleEndChange}
                placeholder="YYYY-MM-DD"
                placeholderTextColor={colors.textMuted}
                keyboardType="numbers-and-punctuation"
                maxLength={10}
                accessibilityLabel="End date"
                testID="end-date-input"
              />
              {endError && (
                <Text
                  style={[styles.errorText, textStyles.caption, { color: colors.error }]}
                >
                  {endError}
                </Text>
              )}
            </View>
          </View>

          {/* Buttons */}
          <View style={[styles.buttons, { marginTop: spacing.lg }]}>
            <Button
              title="Cancel"
              variant="ghost"
              onPress={onClose}
              style={styles.cancelButton}
            />
            <Button
              title="Apply"
              variant="primary"
              onPress={handleApply}
              style={styles.applyButton}
            />
          </View>
        </View>
      </KeyboardAvoidingView>
    </Modal>
  );
}

const styles = StyleSheet.create({
  overlay: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  backdrop: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
  },
  modal: {
    width: '90%',
    maxWidth: 400,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  title: {
    flex: 1,
  },
  closeButton: {
    padding: 4,
  },
  sectionLabel: {
    marginBottom: 4,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  presetsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  presetButton: {
    borderWidth: 1,
  },
  presetText: {
    fontSize: 13,
    fontWeight: '500',
  },
  divider: {
    height: 1,
  },
  dateInputRow: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    marginTop: 8,
  },
  dateInputGroup: {
    // Flex set inline
  },
  inputLabel: {
    marginBottom: 4,
  },
  dateInput: {
    borderWidth: 1,
    fontSize: 14,
    fontVariant: ['tabular-nums'],
  },
  dateArrow: {
    marginTop: 28,
  },
  errorText: {
    marginTop: 4,
  },
  buttons: {
    flexDirection: 'row',
    justifyContent: 'flex-end',
    gap: 12,
  },
  cancelButton: {
    minWidth: 80,
  },
  applyButton: {
    minWidth: 80,
  },
});
