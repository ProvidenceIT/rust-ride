/**
 * RustRide Companion App
 *
 * Mobile companion app for remote workout control, real-time metrics viewing,
 * and ride history access. Connects to the desktop app over local network.
 *
 * @format
 */

import React from 'react';
import { StatusBar, useColorScheme } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { NavigationContainer } from '@react-navigation/native';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { AppNavigator, linking } from '@/navigation';
import { AutoReconnectProvider } from '@/providers';
import { ToastProvider } from '@/hooks';
import { ToastContainer } from '@/components';

// Color palette matching the RustRide desktop app
const Colors = {
  light: {
    background: '#FFFFFF',
  },
  dark: {
    background: '#000000',
  },
};

function App(): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  return (
    <GestureHandlerRootView style={{ flex: 1 }}>
      <SafeAreaProvider>
        <StatusBar
          barStyle={isDarkMode ? 'light-content' : 'dark-content'}
          backgroundColor={colors.background}
        />
        <ToastProvider>
          <AutoReconnectProvider>
            <NavigationContainer
              linking={linking}
              theme={{
                dark: isDarkMode,
                colors: {
                  primary: isDarkMode ? '#0A84FF' : '#007AFF',
                  background: isDarkMode ? '#000000' : '#FFFFFF',
                  card: isDarkMode ? '#1C1C1E' : '#FFFFFF',
                  text: isDarkMode ? '#FFFFFF' : '#1C1C1E',
                  border: isDarkMode ? '#38383A' : '#E5E5EA',
                  notification: isDarkMode ? '#FF453A' : '#FF3B30',
                },
                fonts: {
                  regular: {
                    fontFamily: 'System',
                    fontWeight: '400',
                  },
                  medium: {
                    fontFamily: 'System',
                    fontWeight: '500',
                  },
                  bold: {
                    fontFamily: 'System',
                    fontWeight: '700',
                  },
                  heavy: {
                    fontFamily: 'System',
                    fontWeight: '900',
                  },
                },
              }}>
              <AppNavigator />
            </NavigationContainer>
          </AutoReconnectProvider>
          <ToastContainer testID="toast-container" />
        </ToastProvider>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}

export default App;
