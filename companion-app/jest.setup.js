/**
 * Jest Setup
 *
 * Global mocks and setup for Jest tests.
 */

import 'react-native-gesture-handler/jestSetup';

// Mock react-native-vector-icons
jest.mock('react-native-vector-icons/Ionicons', () => 'Icon');

// Mock Vibration API - directly on the react-native module
const ReactNative = require('react-native');
ReactNative.Vibration = {
  vibrate: jest.fn(),
  cancel: jest.fn(),
};

// Mock @react-navigation/native - provides default navigation context for components using useNavigation
jest.mock('@react-navigation/native', () => {
  const actual = jest.requireActual('@react-navigation/native');
  return {
    ...actual,
    useNavigation: jest.fn(() => ({
      navigate: jest.fn(),
      goBack: jest.fn(),
      setOptions: jest.fn(),
      getState: jest.fn(),
      reset: jest.fn(),
      dispatch: jest.fn(),
      canGoBack: jest.fn(),
      getId: jest.fn(),
      getParent: jest.fn(),
      setParams: jest.fn(),
      addListener: jest.fn(),
      removeListener: jest.fn(),
      isFocused: jest.fn(),
    })),
    useRoute: jest.fn(() => ({
      key: 'test',
      name: 'Test',
      params: undefined,
    })),
    useFocusEffect: jest.fn(),
  };
});
