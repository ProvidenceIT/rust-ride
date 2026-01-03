/**
 * Mock for react-native-camera-kit
 *
 * Provides a testable mock for the Camera component used in QR code scanning.
 */

import React from 'react';
import { View, Text } from 'react-native';

interface OnReadCodeData {
  nativeEvent: {
    codeStringValue: string;
  };
}

interface CameraProps {
  style?: object;
  onReadCode?: (event: OnReadCodeData) => void;
  scanBarcode?: boolean;
  showFrame?: boolean;
  frameColor?: string;
  laserColor?: string;
  onCameraReady?: () => void;
  cameraType?: 'front' | 'back';
  flashMode?: 'on' | 'off' | 'auto';
  ratioOverlay?: string;
  showCaptureButton?: boolean;
  zoomMode?: 'on' | 'off';
  accessibilityLabel?: string;
  testID?: string;
}

// Store for controlling mock behavior in tests
let mockPermissionStatus: boolean | -1 = true;
let mockScanCallback: ((data: string) => void) | null = null;

/**
 * Mock Camera component
 */
const CameraComponent = React.forwardRef<View, CameraProps>(
  function MockCamera({ style, testID, accessibilityLabel, onReadCode, scanBarcode }, ref) {
    // Store the callback for triggering from tests
    React.useEffect(() => {
      if (onReadCode && scanBarcode) {
        mockScanCallback = (data: string) => {
          onReadCode({
            nativeEvent: {
              codeStringValue: data,
            },
          });
        };
      }
      return () => {
        mockScanCallback = null;
      };
    }, [onReadCode, scanBarcode]);

    return (
      <View
        ref={ref}
        style={style}
        testID={testID || 'mock-camera'}
        accessibilityLabel={accessibilityLabel}
      >
        <Text>Mock Camera</Text>
      </View>
    );
  }
);

// Camera type with static methods
interface CameraType extends React.ForwardRefExoticComponent<CameraProps & React.RefAttributes<View>> {
  requestDeviceCameraAuthorization: jest.Mock<Promise<boolean>>;
  checkDeviceCameraAuthorizationStatus: jest.Mock<Promise<boolean | -1>>;
}

// Add static methods to the Camera component
export const Camera = CameraComponent as CameraType;

Camera.requestDeviceCameraAuthorization = jest.fn(
  async (): Promise<boolean> => {
    return mockPermissionStatus === true;
  }
);

Camera.checkDeviceCameraAuthorizationStatus = jest.fn(
  async (): Promise<boolean | -1> => {
    return mockPermissionStatus;
  }
);

/**
 * Helper to set mock permission status in tests
 */
export function setMockPermissionStatus(status: boolean | -1): void {
  mockPermissionStatus = status;
}

/**
 * Helper to simulate scanning a QR code in tests
 */
export function triggerQrScan(data: string): void {
  if (mockScanCallback) {
    mockScanCallback(data);
  }
}

/**
 * Reset all mocks
 */
export function resetMocks(): void {
  mockPermissionStatus = true;
  mockScanCallback = null;
  Camera.requestDeviceCameraAuthorization.mockClear();
  Camera.checkDeviceCameraAuthorizationStatus.mockClear();
}

export default Camera;
