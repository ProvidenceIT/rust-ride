/**
 * Type definitions for react-native-camera-kit
 *
 * Provides QR code scanning capabilities for the companion app.
 */

declare module 'react-native-camera-kit' {
  import type { ViewStyle } from 'react-native';

  /**
   * QR code read event data
   */
  export interface OnReadCodeData {
    /** The scanned code value */
    nativeEvent: {
      codeStringValue: string;
    };
  }

  /**
   * Camera properties for Camera component
   */
  export interface CameraProps {
    /** Container style */
    style?: ViewStyle;
    /** Callback when QR code is read */
    onReadCode?: (event: OnReadCodeData) => void;
    /** Whether to scan barcodes */
    scanBarcode?: boolean;
    /** Show/hide the frame around detected barcodes */
    showFrame?: boolean;
    /** Frame color (hex string) */
    frameColor?: string;
    /** Laser color for scanning animation */
    laserColor?: string;
    /** Callback when camera is ready */
    onCameraReady?: () => void;
    /** Camera type - 'front' or 'back' */
    cameraType?: 'front' | 'back';
    /** Flash mode */
    flashMode?: 'on' | 'off' | 'auto';
    /** Ratio mode for preview */
    ratioOverlay?: string;
    /** Whether to show a capture button */
    showCaptureButton?: boolean;
    /** Whether to enable zoom gestures */
    zoomMode?: 'on' | 'off';
    /** Accessibility label */
    accessibilityLabel?: string;
    /** Test ID for testing */
    testID?: string;
  }

  /**
   * Camera component for capturing photos and scanning barcodes
   */
  export class Camera extends React.Component<CameraProps> {
    /**
     * Capture a photo
     * @returns Promise with photo data
     */
    capture(): Promise<{
      uri: string;
      name: string;
      height: number;
      width: number;
    }>;

    /**
     * Request camera permission
     */
    static requestDeviceCameraAuthorization(): Promise<boolean>;

    /**
     * Check camera permission status
     */
    static checkDeviceCameraAuthorizationStatus(): Promise<boolean | -1>;
  }
}
