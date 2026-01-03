/**
 * ToastContainer Component
 *
 * Renders all active toasts from the toast context.
 * Should be placed at the app's root level, after all navigation.
 *
 * Features:
 * - Renders toasts in a stack
 * - Handles toast dismissal
 * - Positioned above all other content
 */

import React from 'react';
import { StyleSheet, View } from 'react-native';
import { Toast } from './Toast';
import { useToast } from '@/hooks/useToast';

/**
 * ToastContainer props
 */
export interface ToastContainerProps {
  /** Test ID for testing */
  testID?: string;
}

/**
 * ToastContainer Component
 *
 * Renders all active toasts from the toast context.
 * Place this component at the root of your app, after navigation.
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <ToastProvider>
 *       <NavigationContainer>
 *         {...}
 *       </NavigationContainer>
 *       <ToastContainer />
 *     </ToastProvider>
 *   );
 * }
 * ```
 */
export function ToastContainer({ testID }: ToastContainerProps): React.JSX.Element | null {
  const { toasts, dismissToast } = useToast();

  if (toasts.length === 0) {
    return null;
  }

  return (
    <View
      style={styles.container}
      pointerEvents="box-none"
      testID={testID}
    >
      {toasts.map((toast, index) => (
        <Toast
          key={toast.id}
          toast={toast}
          onDismiss={dismissToast}
          style={{ marginTop: index * 8 }}
          testID={`${testID}-toast-${index}`}
        />
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    ...StyleSheet.absoluteFillObject,
    zIndex: 9999,
    pointerEvents: 'box-none',
  },
});
