/**
 * Type definitions for react-native-keep-awake
 *
 * Provides TypeScript types for the react-native-keep-awake library
 * which manages screen wake lock to prevent device from sleeping.
 */

declare module 'react-native-keep-awake' {
  /**
   * React hook to keep the screen awake
   *
   * When this hook is rendered, the screen will stay awake.
   * When unmounted, normal screen timeout behavior resumes.
   *
   * @example
   * ```typescript
   * function WorkoutScreen() {
   *   useKeepAwake();
   *   return <View>...</View>;
   * }
   * ```
   */
  export function useKeepAwake(): void;

  /**
   * Activate screen wake lock
   *
   * Call this to prevent the screen from dimming/sleeping.
   * Must call deactivate() when no longer needed.
   *
   * @example
   * ```typescript
   * import KeepAwake from 'react-native-keep-awake';
   *
   * // During workout
   * KeepAwake.activate();
   *
   * // After workout
   * KeepAwake.deactivate();
   * ```
   */
  export function activate(): void;

  /**
   * Deactivate screen wake lock
   *
   * Call this to allow normal screen timeout behavior.
   */
  export function deactivate(): void;

  /**
   * Default export containing activate and deactivate methods
   */
  const KeepAwake: {
    activate: typeof activate;
    deactivate: typeof deactivate;
  };

  export default KeepAwake;
}
