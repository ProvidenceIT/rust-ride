/**
 * SelectPickerModal Component
 *
 * A modal for selecting from a list of options.
 * Used in Settings screen for units, theme, haptic feedback, etc.
 */

import React, { useCallback } from 'react';
import {
  View,
  Text,
  Modal,
  StyleSheet,
  Pressable,
  FlatList,
} from 'react-native';
import Icon from 'react-native-vector-icons/Ionicons';
import { useTheme } from '@/theme';

/**
 * Option item for the picker
 */
export interface SelectOption<T> {
  /** Value to be returned when selected */
  value: T;
  /** Display label for the option */
  label: string;
  /** Optional description shown below the label */
  description?: string;
  /** Optional icon name */
  icon?: string;
}

/**
 * SelectPickerModal props
 */
export interface SelectPickerModalProps<T> {
  /** Whether the modal is visible */
  visible: boolean;
  /** Modal title */
  title: string;
  /** Available options */
  options: SelectOption<T>[];
  /** Currently selected value */
  selectedValue: T;
  /** Called when an option is selected */
  onSelect: (value: T) => void;
  /** Called when the modal should close */
  onClose: () => void;
}

/**
 * SelectPickerModal Component
 *
 * Displays a list of options in a modal for user selection.
 *
 * @example
 * ```tsx
 * <SelectPickerModal
 *   visible={showPicker}
 *   title="Select Theme"
 *   options={[
 *     { value: 'system', label: 'System', description: 'Follow device theme' },
 *     { value: 'light', label: 'Light' },
 *     { value: 'dark', label: 'Dark' },
 *   ]}
 *   selectedValue={theme}
 *   onSelect={setTheme}
 *   onClose={() => setShowPicker(false)}
 * />
 * ```
 */
export function SelectPickerModal<T>({
  visible,
  title,
  options,
  selectedValue,
  onSelect,
  onClose,
}: SelectPickerModalProps<T>): React.JSX.Element {
  const { colors, spacing, borderRadius, typography } = useTheme();
  const { textStyles } = typography;

  /**
   * Handle option selection
   */
  const handleSelect = useCallback(
    (value: T) => {
      onSelect(value);
      onClose();
    },
    [onSelect, onClose]
  );

  /**
   * Render a single option item
   */
  const renderOption = useCallback(
    ({ item }: { item: SelectOption<T> }) => {
      const isSelected = item.value === selectedValue;

      return (
        <Pressable
          style={({ pressed }) => [
            styles.option,
            {
              backgroundColor: pressed ? colors.surface : colors.background,
              paddingHorizontal: spacing.lg,
              paddingVertical: spacing.md,
              borderBottomWidth: StyleSheet.hairlineWidth,
              borderBottomColor: colors.border,
            },
          ]}
          onPress={() => handleSelect(item.value)}
          accessibilityRole="radio"
          accessibilityState={{ checked: isSelected }}
          accessibilityLabel={item.label}
          accessibilityHint={item.description}
          testID={`option-${String(item.value)}`}
        >
          <View style={styles.optionContent}>
            {item.icon && (
              <Icon
                name={item.icon}
                size={22}
                color={isSelected ? colors.accent : colors.textSecondary}
                style={{ marginRight: spacing.sm }}
              />
            )}
            <View style={styles.optionTextContainer}>
              <Text
                style={[
                  styles.optionLabel,
                  textStyles.body,
                  {
                    color: isSelected ? colors.accent : colors.textPrimary,
                    fontWeight: isSelected ? '600' : '400',
                  },
                ]}
              >
                {item.label}
              </Text>
              {item.description && (
                <Text
                  style={[
                    styles.optionDescription,
                    textStyles.caption,
                    { color: colors.textSecondary },
                  ]}
                >
                  {item.description}
                </Text>
              )}
            </View>
          </View>
          {isSelected && (
            <Icon
              name="checkmark"
              size={22}
              color={colors.accent}
              testID="checkmark-icon"
            />
          )}
        </Pressable>
      );
    },
    [colors, spacing, textStyles, selectedValue, handleSelect]
  );

  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      onRequestClose={onClose}
      statusBarTranslucent
    >
      <Pressable style={styles.overlay} onPress={onClose}>
        <View />
      </Pressable>

      <View style={styles.centeredView}>
        <View
          style={[
            styles.modal,
            {
              backgroundColor: colors.background,
              borderRadius: borderRadius.lg,
            },
          ]}
        >
          {/* Header */}
          <View
            style={[
              styles.header,
              {
                paddingHorizontal: spacing.lg,
                paddingVertical: spacing.md,
                borderBottomWidth: StyleSheet.hairlineWidth,
                borderBottomColor: colors.border,
              },
            ]}
          >
            <Text
              style={[
                styles.title,
                textStyles.sectionTitle,
                { color: colors.textPrimary },
              ]}
            >
              {title}
            </Text>
            <Pressable
              onPress={onClose}
              style={[styles.closeButton, { padding: spacing.xs }]}
              accessibilityRole="button"
              accessibilityLabel="Close"
              hitSlop={{ top: 10, bottom: 10, left: 10, right: 10 }}
            >
              <Icon name="close" size={24} color={colors.textSecondary} />
            </Pressable>
          </View>

          {/* Options List */}
          <FlatList
            data={options}
            renderItem={renderOption}
            keyExtractor={(item) => String(item.value)}
            style={styles.list}
            showsVerticalScrollIndicator={false}
            accessibilityRole="radiogroup"
          />
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  overlay: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
  },
  centeredView: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 20,
  },
  modal: {
    width: '100%',
    maxWidth: 340,
    maxHeight: '70%',
    overflow: 'hidden',
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
    marginLeft: 8,
  },
  list: {
    flexGrow: 0,
  },
  option: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  optionContent: {
    flex: 1,
    flexDirection: 'row',
    alignItems: 'center',
  },
  optionTextContainer: {
    flex: 1,
  },
  optionLabel: {
    // Styles applied inline
  },
  optionDescription: {
    marginTop: 2,
  },
});
