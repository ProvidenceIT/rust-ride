/**
 * Settings Screen
 *
 * App settings including connection, display preferences,
 * and other configuration options.
 */

import React from 'react';
import { StyleSheet, Text, View, useColorScheme, ScrollView, Switch, TouchableOpacity } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import type { MainTabScreenProps } from '@/navigation/types';

const Colors = {
  light: {
    background: '#FFFFFF',
    surface: '#F5F5F5',
    primary: '#007AFF',
    text: '#1C1C1E',
    textSecondary: '#8E8E93',
    border: '#E5E5EA',
  },
  dark: {
    background: '#000000',
    surface: '#1C1C1E',
    primary: '#0A84FF',
    text: '#FFFFFF',
    textSecondary: '#8E8E93',
    border: '#38383A',
  },
};

type Props = MainTabScreenProps<'Settings'>;

interface SettingRowProps {
  label: string;
  value?: string;
  hasChevron?: boolean;
  onPress?: () => void;
  colors: typeof Colors.light;
}

function SettingRow({ label, value, hasChevron, onPress, colors }: SettingRowProps): React.JSX.Element {
  return (
    <TouchableOpacity
      style={[styles.settingRow, { borderBottomColor: colors.border }]}
      onPress={onPress}
      disabled={!onPress}
      activeOpacity={0.7}
    >
      <Text style={[styles.settingLabel, { color: colors.text }]}>{label}</Text>
      <View style={styles.settingValueContainer}>
        {value && <Text style={[styles.settingValue, { color: colors.textSecondary }]}>{value}</Text>}
        {hasChevron && <Text style={[styles.chevron, { color: colors.textSecondary }]}>{'>'}</Text>}
      </View>
    </TouchableOpacity>
  );
}

interface SettingToggleRowProps {
  label: string;
  value: boolean;
  onValueChange: (value: boolean) => void;
  colors: typeof Colors.light;
}

function SettingToggleRow({ label, value, onValueChange, colors }: SettingToggleRowProps): React.JSX.Element {
  return (
    <View style={[styles.settingRow, { borderBottomColor: colors.border }]}>
      <Text style={[styles.settingLabel, { color: colors.text }]}>{label}</Text>
      <Switch
        value={value}
        onValueChange={onValueChange}
        trackColor={{ false: colors.border, true: colors.primary }}
      />
    </View>
  );
}

export function SettingsScreen(_props: Props): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  // Placeholder settings state
  const [keepAwake, setKeepAwake] = React.useState(true);

  return (
    <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]} edges={['top']}>
      <View style={styles.header}>
        <Text style={[styles.title, { color: colors.text }]}>Settings</Text>
      </View>

      <ScrollView style={styles.scrollView} showsVerticalScrollIndicator={false}>
        {/* Connection section */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Connection</Text>
          <View style={[styles.sectionContent, { backgroundColor: colors.surface }]}>
            <SettingRow
              label="Server"
              value="Not connected"
              hasChevron
              colors={colors}
            />
            <SettingRow
              label="Scan QR Code"
              hasChevron
              colors={colors}
            />
          </View>
        </View>

        {/* Display section */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Display</Text>
          <View style={[styles.sectionContent, { backgroundColor: colors.surface }]}>
            <SettingRow
              label="Units"
              value="Metric"
              hasChevron
              colors={colors}
            />
            <SettingRow
              label="Theme"
              value="System"
              hasChevron
              colors={colors}
            />
            <SettingToggleRow
              label="Keep Screen Awake"
              value={keepAwake}
              onValueChange={setKeepAwake}
              colors={colors}
            />
          </View>
        </View>

        {/* Feedback section */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>Feedback</Text>
          <View style={[styles.sectionContent, { backgroundColor: colors.surface }]}>
            <SettingRow
              label="Haptic Feedback"
              value="Medium"
              hasChevron
              colors={colors}
            />
          </View>
        </View>

        {/* About section */}
        <View style={styles.section}>
          <Text style={[styles.sectionTitle, { color: colors.textSecondary }]}>About</Text>
          <View style={[styles.sectionContent, { backgroundColor: colors.surface }]}>
            <SettingRow
              label="Version"
              value="1.0.0"
              colors={colors}
            />
          </View>
        </View>

        <View style={styles.footer} />
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  scrollView: {
    flex: 1,
  },
  section: {
    marginBottom: 24,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 8,
    paddingHorizontal: 16,
  },
  sectionContent: {
    borderRadius: 12,
    marginHorizontal: 16,
    overflow: 'hidden',
  },
  settingRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderBottomWidth: StyleSheet.hairlineWidth,
  },
  settingLabel: {
    fontSize: 16,
  },
  settingValueContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 4,
  },
  settingValue: {
    fontSize: 16,
  },
  chevron: {
    fontSize: 18,
    fontWeight: '300',
  },
  footer: {
    height: 40,
  },
});
