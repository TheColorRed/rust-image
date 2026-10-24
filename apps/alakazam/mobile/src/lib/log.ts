/**
 * Logs a message to the console in development mode.
 * @param key The key or label for the log message.
 * @param message The message or data to be logged.
 */
export function devLog(key: string, ...message: any[]): void {
  if (__DEV__) console.log(`[${key}]`, ...message);
}
/**
 * Logs an error message to the console in development mode.
 * @param key The key or label for the error message.
 * @param message The error message or data to be logged.
 */
export function devError(key: string, ...message: any[]): void {
  if (__DEV__) console.error(`[${key}]`, ...message);
}
/**
 * Logs a warning message to the console in development mode.
 * @param key The key or label for the warning message.
 * @param message The warning message or data to be logged.
 */
export function devWarn(key: string, ...message: any[]): void {
  if (__DEV__) console.warn(`[${key}]`, ...message);
}
/**
 * Logs an informational message to the console in development mode.
 * @param key The key or label for the informational message.
 * @param message The informational message or data to be logged.
 */
export function devInfo(key: string, ...message: any[]): void {
  if (__DEV__) console.info(`[${key}]`, ...message);
}
/**
 * Logs a debug message to the console in development mode.
 * @param key The key or label for the debug message.
 * @param message The debug message or data to be logged.
 */
export function devDebug(key: string, ...message: any[]): void {
  if (__DEV__) console.debug(`[${key}]`, ...message);
}
/**
 * Logs a message to the console.
 * @param key The key or label for the log message.
 * @param message The message or data to be logged.
 */
export function log(key: string, ...message: any[]): void {
  console.log(`[${key}]`, ...message);
}
/**
 * Logs an error message to the console.
 * @param key The key or label for the error message.
 * @param message The error message or data to be logged.
 */
export function error(key: string, ...message: any[]): void {
  console.error(`[${key}]`, ...message);
}
/**
 * Logs a warning message to the console.
 * @param key The key or label for the warning message.
 * @param message The warning message or data to be logged.
 */
export function warn(key: string, ...message: any[]): void {
  console.warn(`[${key}]`, ...message);
}
/**
 * Logs an informational message to the console.
 * @param key The key or label for the informational message.
 * @param message The informational message or data to be logged.
 */
export function info(key: string, ...message: any[]): void {
  console.info(`[${key}]`, ...message);
}
/**
 * Logs a debug message to the console.
 * @param key The key or label for the debug message.
 * @param message The debug message or data to be logged.
 */
export function debug(key: string, ...message: any[]): void {
  console.debug(`[${key}]`, ...message);
}
