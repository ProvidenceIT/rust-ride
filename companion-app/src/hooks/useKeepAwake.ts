/**
 * useKeepAwake Hook
 *
 * Manages screen wake lock during active workout sessions based on user settings.
 * Uses react-native-keep-awake to prevent the device from sleeping.
 */

import { useEffect, useRef } from 'react';
import KeepAwake from 'react-native-keep-awake';
import { useSettingsStore, selectKeepScreenAwake } from '@/stores/settingsStore';
import { useSessionStore, selectIsSessionActive } from '@/stores/sessionStore';

/**
 * Return type for the useKeepAwake hook
 */
export interface UseKeepAwakeReturn {
  /** Whether keep awake is currently active */
  isKeepAwakeActive: boolean;
  /** Whether the keep awake setting is enabled */
  isSettingEnabled: boolean;
  /** Whether there's an active session */
  hasActiveSession: boolean;
}

/**
 * useKeepAwake hook
 *
 * Automatically activates screen wake lock when:
 * 1. The keepScreenAwake setting is enabled
 * 2. There is an active workout/ride session
 *
 * The wake lock is released when either condition becomes false.
 *
 * @example
 * ```tsx
 * function App() {
 *   // Simply using the hook in a parent component activates the feature
 *   useKeepAwake();
 *   return <AppNavigator />;
 * }
 * ```
 *
 * @returns Object containing wake lock state information
 */
export function useKeepAwake(): UseKeepAwakeReturn {
  const keepScreenAwakeSetting = useSettingsStore(selectKeepScreenAwake);
  const isSessionActive = useSessionStore(selectIsSessionActive);

  // Track whether we currently have an active wake lock
  const isActive = keepScreenAwakeSetting && isSessionActive;

  // Use ref to track previous state and avoid unnecessary calls
  const wasActiveRef = useRef(false);

  useEffect(() => {
    const shouldActivate = keepScreenAwakeSetting && isSessionActive;

    if (shouldActivate && !wasActiveRef.current) {
      // Activate keep awake
      KeepAwake.activate();
      wasActiveRef.current = true;
    } else if (!shouldActivate && wasActiveRef.current) {
      // Deactivate keep awake
      KeepAwake.deactivate();
      wasActiveRef.current = false;
    }

    // Cleanup on unmount - always deactivate
    return () => {
      if (wasActiveRef.current) {
        KeepAwake.deactivate();
        wasActiveRef.current = false;
      }
    };
  }, [keepScreenAwakeSetting, isSessionActive]);

  return {
    isKeepAwakeActive: isActive,
    isSettingEnabled: keepScreenAwakeSetting,
    hasActiveSession: isSessionActive,
  };
}

/**
 * KeepAwakeWrapper Component
 *
 * A component wrapper that manages keep awake state.
 * Useful when you need to conditionally render based on keep awake state.
 *
 * @example
 * ```tsx
 * function WorkoutScreen() {
 *   return (
 *     <KeepAwakeWrapper>
 *       {({ isKeepAwakeActive }) => (
 *         <View>
 *           {isKeepAwakeActive && <Text>Screen will stay on</Text>}
 *           <WorkoutContent />
 *         </View>
 *       )}
 *     </KeepAwakeWrapper>
 *   );
 * }
 * ```
 */
export interface KeepAwakeWrapperProps {
  /** Render function that receives keep awake state */
  children: (state: UseKeepAwakeReturn) => React.ReactNode;
}
