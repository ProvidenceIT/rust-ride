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
