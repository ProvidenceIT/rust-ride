/**
 * RustRide Companion App
 *
 * Mobile companion app for remote workout control, real-time metrics viewing,
 * and ride history access. Connects to the desktop app over local network.
 *
 * @format
 */

import React from 'react';
import { StatusBar, StyleSheet, Text, View, useColorScheme } from 'react-native';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';

// Color palette matching the RustRide desktop app
const Colors = {
  light: {
    background: '#FFFFFF',
    surface: '#F5F5F5',
    primary: '#007AFF',
    text: '#1C1C1E',
    textSecondary: '#8E8E93',
  },
  dark: {
    background: '#000000',
    surface: '#1C1C1E',
    primary: '#0A84FF',
    text: '#FFFFFF',
    textSecondary: '#8E8E93',
  },
};

function App(): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  return (
    <SafeAreaProvider>
      <StatusBar
        barStyle={isDarkMode ? 'light-content' : 'dark-content'}
        backgroundColor={colors.background}
      />
      <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]}>
        <View style={styles.content}>
          <Text style={[styles.title, { color: colors.text }]}>RustRide Companion</Text>
          <Text style={[styles.subtitle, { color: colors.textSecondary }]}>
            Connect to your desktop app to control workouts and view metrics
          </Text>
          <View style={[styles.statusCard, { backgroundColor: colors.surface }]}>
            <Text style={[styles.statusText, { color: colors.textSecondary }]}>Not connected</Text>
          </View>
        </View>
      </SafeAreaView>
    </SafeAreaProvider>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  content: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 20,
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 16,
    textAlign: 'center',
    marginBottom: 40,
    paddingHorizontal: 20,
  },
  statusCard: {
    paddingVertical: 16,
    paddingHorizontal: 32,
    borderRadius: 12,
  },
  statusText: {
    fontSize: 14,
  },
});

export default App;
