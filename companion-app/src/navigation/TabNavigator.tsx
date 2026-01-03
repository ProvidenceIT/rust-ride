/**
 * Tab Navigator
 *
 * Bottom tab navigation for main app screens:
 * Dashboard, Workout, History, Settings
 */

import React from 'react';
import { StyleSheet, useColorScheme } from 'react-native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import Icon from 'react-native-vector-icons/Ionicons';
import type { MainTabParamList } from './types';
import {
  DashboardScreen,
  WorkoutScreen,
  HistoryScreen,
  SettingsScreen,
} from '@/screens';

const Tab = createBottomTabNavigator<MainTabParamList>();

const Colors = {
  light: {
    background: '#FFFFFF',
    border: '#E5E5EA',
    tabActive: '#007AFF',
    tabInactive: '#8E8E93',
  },
  dark: {
    background: '#1C1C1E',
    border: '#38383A',
    tabActive: '#0A84FF',
    tabInactive: '#8E8E93',
  },
};

type TabIconProps = {
  focused: boolean;
  color: string;
  size: number;
};

/**
 * Returns the icon name for a given tab route.
 */
function getIconName(routeName: keyof MainTabParamList, focused: boolean): string {
  switch (routeName) {
    case 'Dashboard':
      return focused ? 'speedometer' : 'speedometer-outline';
    case 'Workout':
      return focused ? 'barbell' : 'barbell-outline';
    case 'History':
      return focused ? 'time' : 'time-outline';
    case 'Settings':
      return focused ? 'settings' : 'settings-outline';
    default:
      return 'help-outline';
  }
}

/**
 * Tab bar icon component for the given route.
 */
function TabBarIcon({
  routeName,
  focused,
  color,
  size,
}: TabIconProps & { routeName: keyof MainTabParamList }): React.JSX.Element {
  const iconName = getIconName(routeName, focused);
  return <Icon name={iconName} size={size} color={color} />;
}

/**
 * Creates a tab bar icon renderer for the specified route.
 */
function createTabBarIcon(routeName: keyof MainTabParamList) {
  return function TabIcon(props: TabIconProps): React.JSX.Element {
    return <TabBarIcon routeName={routeName} {...props} />;
  };
}

export function TabNavigator(): React.JSX.Element {
  const isDarkMode = useColorScheme() === 'dark';
  const colors = isDarkMode ? Colors.dark : Colors.light;

  return (
    <Tab.Navigator
      initialRouteName="Dashboard"
      screenOptions={{
        headerShown: false,
        tabBarActiveTintColor: colors.tabActive,
        tabBarInactiveTintColor: colors.tabInactive,
        tabBarStyle: [
          styles.tabBar,
          {
            backgroundColor: colors.background,
            borderTopColor: colors.border,
          },
        ],
        tabBarLabelStyle: styles.tabBarLabel,
        tabBarHideOnKeyboard: true,
      }}
    >
      <Tab.Screen
        name="Dashboard"
        component={DashboardScreen}
        options={{
          tabBarLabel: 'Dashboard',
          tabBarIcon: createTabBarIcon('Dashboard'),
        }}
      />
      <Tab.Screen
        name="Workout"
        component={WorkoutScreen}
        options={{
          tabBarLabel: 'Workout',
          tabBarIcon: createTabBarIcon('Workout'),
        }}
      />
      <Tab.Screen
        name="History"
        component={HistoryScreen}
        options={{
          tabBarLabel: 'History',
          tabBarIcon: createTabBarIcon('History'),
        }}
      />
      <Tab.Screen
        name="Settings"
        component={SettingsScreen}
        options={{
          tabBarLabel: 'Settings',
          tabBarIcon: createTabBarIcon('Settings'),
        }}
      />
    </Tab.Navigator>
  );
}

const styles = StyleSheet.create({
  tabBar: {
    borderTopWidth: StyleSheet.hairlineWidth,
    paddingTop: 8,
    height: 88,
  },
  tabBarLabel: {
    fontSize: 11,
    fontWeight: '500',
    marginTop: 4,
  },
});
