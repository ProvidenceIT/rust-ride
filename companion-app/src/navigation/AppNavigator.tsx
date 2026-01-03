/**
 * App Navigator
 *
 * Root stack navigator containing the tab navigator and modal screens.
 * Handles navigation between main tabs and detail/modal screens.
 */

import React from 'react';
import { useColorScheme } from 'react-native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import type { RootStackParamList } from './types';
import { TabNavigator } from './TabNavigator';
import { RideDetailScreen, ConnectionScreen } from '@/screens';

const Stack = createNativeStackNavigator<RootStackParamList>();

const Colors = {
  light: {
    background: '#FFFFFF',
    text: '#1C1C1E',
    primary: '#007AFF',
  },
  dark: {
    background: '#000000',
    text: '#FFFFFF',
    primary: '#0A84FF',
  },
};

export function AppNavigator(): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  return (
    <Stack.Navigator
      initialRouteName="Main"
      screenOptions={{
        headerShown: true,
        headerBackTitle: '',
        headerTintColor: colors.primary,
        headerStyle: {
          backgroundColor: colors.background,
        },
        headerTitleStyle: {
          color: colors.text,
          fontWeight: '600',
        },
        contentStyle: {
          backgroundColor: colors.background,
        },
        animation: 'slide_from_right',
      }}
    >
      <Stack.Screen
        name="Main"
        component={TabNavigator}
        options={{
          headerShown: false,
        }}
      />
      <Stack.Screen
        name="RideDetail"
        component={RideDetailScreen}
        options={{
          title: 'Ride Details',
          animation: 'slide_from_right',
        }}
      />
      <Stack.Screen
        name="Connection"
        component={ConnectionScreen}
        options={{
          title: 'Connect',
          presentation: 'modal',
          animation: 'slide_from_bottom',
        }}
      />
    </Stack.Navigator>
  );
}
