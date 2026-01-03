/**
 * Settings Screen
 *
 * App settings including connection, display preferences,
 * and other configuration options.
 */

import React, { useCallback, useEffect, useState } from 'react';
import {
  StyleSheet,
  Text,
  View,
  ScrollView,
  Switch,
  Pressable,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import Icon from 'react-native-vector-icons/Ionicons';
import type { MainTabScreenProps, RootStackNavigationProp } from '@/navigation/types';
import { useTheme, useThemeContext } from '@/theme';
import {
  useSettingsStore,
  selectUnits,
  selectKeepScreenAwake,
  selectHapticFeedback,
  selectTheme,
  type UnitSystem,
  type HapticIntensity,
  type ThemePreference,
} from '@/stores/settingsStore';
import {
  useConnectionStore,
  selectConnectionStatus,
  selectCurrentServer,
  selectIsAuthenticated,
} from '@/stores/connectionStore';
import { SelectPickerModal, type SelectOption } from '@/components/SelectPickerModal';

// App version (in a real app, this would come from the app config)
const APP_VERSION = '1.0.0';

type Props = MainTabScreenProps<'Settings'>;

/**
 * Section header component
 */
interface SectionHeaderProps {
  title: string;
}

function SectionHeader({ title }: SectionHeaderProps): React.JSX.Element {
  const { colors, spacing, typography } = useTheme();
  const { textStyles } = typography;

  return (
    <Text
      style={[
        styles.sectionTitle,
        textStyles.label,
        {
          color: colors.textSecondary,
          paddingHorizontal: spacing.lg,
          marginBottom: spacing.xs,
        },
      ]}
    >
      {title}
    </Text>
  );
}

/**
 * Setting row with optional value and chevron
 */
interface SettingRowProps {
  label: string;
  value?: string;
  icon?: string;
  hasChevron?: boolean;
  onPress?: () => void;
  isFirst?: boolean;
  isLast?: boolean;
  testID?: string;
}

function SettingRow({
  label,
  value,
  icon,
  hasChevron = true,
  onPress,
  isFirst = false,
  isLast = false,
  testID,
}: SettingRowProps): React.JSX.Element {
  const { colors, spacing, borderRadius, typography } = useTheme();
  const { textStyles } = typography;

  return (
    <Pressable
      style={({ pressed }) => [
        styles.settingRow,
        {
          backgroundColor: pressed && onPress ? colors.elevated : colors.surface,
          borderBottomWidth: isLast ? 0 : StyleSheet.hairlineWidth,
          borderBottomColor: colors.border,
          paddingVertical: spacing.md,
          paddingHorizontal: spacing.lg,
          borderTopLeftRadius: isFirst ? borderRadius.md : 0,
          borderTopRightRadius: isFirst ? borderRadius.md : 0,
          borderBottomLeftRadius: isLast ? borderRadius.md : 0,
          borderBottomRightRadius: isLast ? borderRadius.md : 0,
        },
      ]}
      onPress={onPress}
      disabled={!onPress}
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityValue={{ text: value }}
      accessibilityHint={hasChevron ? 'Opens settings for ' + label : undefined}
      testID={testID}
    >
      <View style={styles.settingRowContent}>
        {icon && (
          <Icon
            name={icon}
            size={22}
            color={colors.accent}
            style={{ marginRight: spacing.sm }}
          />
        )}
        <Text style={[styles.settingLabel, textStyles.body, { color: colors.textPrimary }]}>
          {label}
        </Text>
      </View>
      <View style={styles.settingValueContainer}>
        {value && (
          <Text
            style={[styles.settingValue, textStyles.body, { color: colors.textSecondary }]}
          >
            {value}
          </Text>
        )}
        {hasChevron && onPress && (
          <Icon
            name="chevron-forward"
            size={18}
            color={colors.textSecondary}
            style={{ marginLeft: spacing.xs }}
          />
        )}
      </View>
    </Pressable>
  );
}

/**
 * Setting row with toggle switch
 */
interface SettingToggleRowProps {
  label: string;
  icon?: string;
  value: boolean;
  onValueChange: (value: boolean) => void;
  isFirst?: boolean;
  isLast?: boolean;
  testID?: string;
}

function SettingToggleRow({
  label,
  icon,
  value,
  onValueChange,
  isFirst = false,
  isLast = false,
  testID,
}: SettingToggleRowProps): React.JSX.Element {
  const { colors, spacing, borderRadius, typography } = useTheme();
  const { textStyles } = typography;

  return (
    <View
      style={[
        styles.settingRow,
        {
          backgroundColor: colors.surface,
          borderBottomWidth: isLast ? 0 : StyleSheet.hairlineWidth,
          borderBottomColor: colors.border,
          paddingVertical: spacing.md,
          paddingHorizontal: spacing.lg,
          borderTopLeftRadius: isFirst ? borderRadius.md : 0,
          borderTopRightRadius: isFirst ? borderRadius.md : 0,
          borderBottomLeftRadius: isLast ? borderRadius.md : 0,
          borderBottomRightRadius: isLast ? borderRadius.md : 0,
        },
      ]}
      accessibilityRole="switch"
      accessibilityLabel={label}
      accessibilityState={{ checked: value }}
    >
      <View style={styles.settingRowContent}>
        {icon && (
          <Icon
            name={icon}
            size={22}
            color={colors.accent}
            style={{ marginRight: spacing.sm }}
          />
        )}
        <Text style={[styles.settingLabel, textStyles.body, { color: colors.textPrimary }]}>
          {label}
        </Text>
      </View>
      <Switch
        value={value}
        onValueChange={onValueChange}
        trackColor={{ false: colors.border, true: colors.accent }}
        thumbColor={colors.textInverse}
        ios_backgroundColor={colors.border}
        testID={testID}
      />
    </View>
  );
}

/**
 * Section container component
 */
interface SectionContainerProps {
  children: React.ReactNode;
}

function SectionContainer({ children }: SectionContainerProps): React.JSX.Element {
  const { colors, spacing, borderRadius } = useTheme();

  return (
    <View
      style={[
        styles.sectionContent,
        {
          backgroundColor: colors.surface,
          borderRadius: borderRadius.md,
          marginHorizontal: spacing.lg,
          overflow: 'hidden',
        },
      ]}
    >
      {children}
    </View>
  );
}

// ============================================================
// Picker Options
// ============================================================

const UNIT_OPTIONS: SelectOption<UnitSystem>[] = [
  {
    value: 'metric',
    label: 'Metric',
    description: 'Kilometers, km/h',
    icon: 'speedometer-outline',
  },
  {
    value: 'imperial',
    label: 'Imperial',
    description: 'Miles, mph',
    icon: 'speedometer-outline',
  },
];

const THEME_OPTIONS: SelectOption<ThemePreference>[] = [
  {
    value: 'system',
    label: 'System',
    description: 'Follow device theme',
    icon: 'phone-portrait-outline',
  },
  {
    value: 'light',
    label: 'Light',
    description: 'Always use light theme',
    icon: 'sunny-outline',
  },
  {
    value: 'dark',
    label: 'Dark',
    description: 'Always use dark theme',
    icon: 'moon-outline',
  },
];

const HAPTIC_OPTIONS: SelectOption<HapticIntensity>[] = [
  {
    value: 'off',
    label: 'Off',
    description: 'No haptic feedback',
  },
  {
    value: 'light',
    label: 'Light',
    description: 'Subtle feedback',
  },
  {
    value: 'medium',
    label: 'Medium',
    description: 'Standard feedback',
  },
  {
    value: 'strong',
    label: 'Strong',
    description: 'Prominent feedback',
  },
];

/**
 * Get display label for unit system
 */
function getUnitLabel(units: UnitSystem): string {
  return units === 'metric' ? 'Metric' : 'Imperial';
}

/**
 * Get display label for theme preference
 */
function getThemeLabel(theme: ThemePreference): string {
  switch (theme) {
    case 'system':
      return 'System';
    case 'light':
      return 'Light';
    case 'dark':
      return 'Dark';
  }
}

/**
 * Get display label for haptic intensity
 */
function getHapticLabel(intensity: HapticIntensity): string {
  switch (intensity) {
    case 'off':
      return 'Off';
    case 'light':
      return 'Light';
    case 'medium':
      return 'Medium';
    case 'strong':
      return 'Strong';
  }
}

/**
 * Get connection status text
 */
function getConnectionStatusText(
  status: string,
  isAuthenticated: boolean,
  serverName?: string
): string {
  if (status === 'authenticated' || (status === 'connected' && isAuthenticated)) {
    return serverName || 'Connected';
  }
  if (status === 'connecting') {
    return 'Connecting...';
  }
  return 'Not connected';
}

/**
 * Settings Screen Component
 *
 * Displays app settings organized in sections:
 * - Connection: Server connection status and settings
 * - Display: Units, theme, screen awake
 * - Feedback: Haptic feedback settings
 * - About: App version info
 */
export function SettingsScreen(_props: Props): React.JSX.Element {
  const { colors, spacing } = useTheme();
  const { setThemeMode } = useThemeContext();
  const navigation = useNavigation<RootStackNavigationProp>();

  // Settings store
  const units = useSettingsStore(selectUnits);
  const keepScreenAwake = useSettingsStore(selectKeepScreenAwake);
  const hapticFeedback = useSettingsStore(selectHapticFeedback);
  const theme = useSettingsStore(selectTheme);
  const loadSettings = useSettingsStore((state) => state.loadSettings);
  const setUnits = useSettingsStore((state) => state.setUnits);
  const setKeepScreenAwake = useSettingsStore((state) => state.setKeepScreenAwake);
  const setHapticFeedback = useSettingsStore((state) => state.setHapticFeedback);
  const setThemeSetting = useSettingsStore((state) => state.setTheme);

  // Connection store
  const connectionStatus = useConnectionStore(selectConnectionStatus);
  const currentServer = useConnectionStore(selectCurrentServer);
  const isAuthenticated = useConnectionStore(selectIsAuthenticated);

  // Modal visibility state
  const [showUnitsPicker, setShowUnitsPicker] = useState(false);
  const [showThemePicker, setShowThemePicker] = useState(false);
  const [showHapticPicker, setShowHapticPicker] = useState(false);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Handle navigation to connection screen
  const handleConnectionPress = useCallback(() => {
    navigation.navigate('Connection');
  }, [navigation]);

  // Handle unit selection
  const handleUnitSelect = useCallback(
    async (value: UnitSystem) => {
      await setUnits(value);
    },
    [setUnits]
  );

  // Handle theme selection
  const handleThemeSelect = useCallback(
    async (value: ThemePreference) => {
      await setThemeSetting(value);
      setThemeMode(value);
    },
    [setThemeSetting, setThemeMode]
  );

  // Handle haptic feedback selection
  const handleHapticSelect = useCallback(
    async (value: HapticIntensity) => {
      await setHapticFeedback(value);
    },
    [setHapticFeedback]
  );

  // Handle keep screen awake toggle
  const handleKeepScreenAwakeToggle = useCallback(
    async (value: boolean) => {
      await setKeepScreenAwake(value);
    },
    [setKeepScreenAwake]
  );

  const connectionStatusText = getConnectionStatusText(
    connectionStatus,
    isAuthenticated,
    currentServer?.name
  );

  return (
    <SafeAreaView
      style={[styles.container, { backgroundColor: colors.background }]}
      edges={['top']}
    >
      {/* Header */}
      <View style={[styles.header, { paddingHorizontal: spacing.lg, paddingVertical: spacing.md }]}>
        <Text style={[styles.title, { color: colors.textPrimary }]}>Settings</Text>
      </View>

      <ScrollView
        style={styles.scrollView}
        contentContainerStyle={[styles.scrollContent, { paddingBottom: spacing['3xl'] }]}
        showsVerticalScrollIndicator={false}
      >
        {/* Connection Section */}
        <View style={[styles.section, { marginTop: spacing.sm }]}>
          <SectionHeader title="CONNECTION" />
          <SectionContainer>
            <SettingRow
              label="Server"
              value={connectionStatusText}
              icon="server-outline"
              onPress={handleConnectionPress}
              isFirst
              isLast
              testID="setting-server"
            />
          </SectionContainer>
        </View>

        {/* Display Section */}
        <View style={[styles.section, { marginTop: spacing.lg }]}>
          <SectionHeader title="DISPLAY" />
          <SectionContainer>
            <SettingRow
              label="Units"
              value={getUnitLabel(units)}
              icon="analytics-outline"
              onPress={() => setShowUnitsPicker(true)}
              isFirst
              testID="setting-units"
            />
            <SettingRow
              label="Theme"
              value={getThemeLabel(theme)}
              icon="color-palette-outline"
              onPress={() => setShowThemePicker(true)}
              testID="setting-theme"
            />
            <SettingToggleRow
              label="Keep Screen Awake"
              icon="sunny-outline"
              value={keepScreenAwake}
              onValueChange={handleKeepScreenAwakeToggle}
              isLast
              testID="setting-keep-awake"
            />
          </SectionContainer>
        </View>

        {/* Feedback Section */}
        <View style={[styles.section, { marginTop: spacing.lg }]}>
          <SectionHeader title="FEEDBACK" />
          <SectionContainer>
            <SettingRow
              label="Haptic Feedback"
              value={getHapticLabel(hapticFeedback)}
              icon="pulse-outline"
              onPress={() => setShowHapticPicker(true)}
              isFirst
              isLast
              testID="setting-haptic"
            />
          </SectionContainer>
        </View>

        {/* About Section */}
        <View style={[styles.section, { marginTop: spacing.lg }]}>
          <SectionHeader title="ABOUT" />
          <SectionContainer>
            <SettingRow
              label="Version"
              value={APP_VERSION}
              icon="information-circle-outline"
              hasChevron={false}
              isFirst
              isLast
              testID="setting-version"
            />
          </SectionContainer>
        </View>
      </ScrollView>

      {/* Units Picker Modal */}
      <SelectPickerModal
        visible={showUnitsPicker}
        title="Select Units"
        options={UNIT_OPTIONS}
        selectedValue={units}
        onSelect={handleUnitSelect}
        onClose={() => setShowUnitsPicker(false)}
      />

      {/* Theme Picker Modal */}
      <SelectPickerModal
        visible={showThemePicker}
        title="Select Theme"
        options={THEME_OPTIONS}
        selectedValue={theme}
        onSelect={handleThemeSelect}
        onClose={() => setShowThemePicker(false)}
      />

      {/* Haptic Feedback Picker Modal */}
      <SelectPickerModal
        visible={showHapticPicker}
        title="Haptic Feedback"
        options={HAPTIC_OPTIONS}
        selectedValue={hapticFeedback}
        onSelect={handleHapticSelect}
        onClose={() => setShowHapticPicker(false)}
      />
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    // Padding applied inline
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  scrollView: {
    flex: 1,
  },
  scrollContent: {
    // Padding applied inline
  },
  section: {
    // Margin applied inline
  },
  sectionTitle: {
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    fontWeight: '600',
    fontSize: 13,
  },
  sectionContent: {
    // Styles applied inline
  },
  settingRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  settingRowContent: {
    flexDirection: 'row',
    alignItems: 'center',
    flex: 1,
  },
  settingLabel: {
    flex: 1,
  },
  settingValueContainer: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  settingValue: {
    // Styles applied inline
  },
});
