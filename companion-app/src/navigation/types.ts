/**
 * Navigation Type Definitions
 *
 * Type-safe navigation using React Navigation typed params.
 */

import type { NavigatorScreenParams } from '@react-navigation/native';
import type { NativeStackScreenProps } from '@react-navigation/native-stack';
import type { BottomTabScreenProps } from '@react-navigation/bottom-tabs';

/**
 * Root stack param list - contains the tab navigator and modal screens
 */
export type RootStackParamList = {
  Main: NavigatorScreenParams<MainTabParamList>;
  RideDetail: { rideId: string };
  Connection: undefined;
};

/**
 * Main tab navigator param list - bottom tab screens
 */
export type MainTabParamList = {
  Dashboard: undefined;
  Workout: undefined;
  History: undefined;
  Settings: undefined;
};

/**
 * History stack param list - for nested stack navigation in History tab
 */
export type HistoryStackParamList = {
  HistoryList: undefined;
  HistoryDetail: { rideId: string };
};

/**
 * Root stack screen props
 */
export type RootStackScreenProps<T extends keyof RootStackParamList> =
  NativeStackScreenProps<RootStackParamList, T>;

/**
 * Main tab screen props
 */
export type MainTabScreenProps<T extends keyof MainTabParamList> =
  BottomTabScreenProps<MainTabParamList, T>;

/**
 * History stack screen props
 */
export type HistoryStackScreenProps<T extends keyof HistoryStackParamList> =
  NativeStackScreenProps<HistoryStackParamList, T>;

// Extend the global navigation namespace for useNavigation hook typing
// This is required by React Navigation for type-safe navigation
declare global {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace ReactNavigation {
    interface RootParamList extends RootStackParamList {}
  }
}
