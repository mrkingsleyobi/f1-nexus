/**
 * F1 Nexus NAPI-RS bindings for Node.js
 */

/**
 * Get F1 Nexus version
 */
export function version(): string;

/**
 * Get list of supported circuits
 */
export function getCircuits(): string[];

/**
 * Get list of tire compounds
 */
export function getTireCompounds(): string[];

/**
 * Optimize pit stop strategy
 * @param paramsJson - JSON string with optimization parameters
 * @returns JSON string with optimized strategy
 */
export function optimizeStrategy(paramsJson: string): string;

/**
 * Simulate race with given strategy
 * @param paramsJson - JSON string with simulation parameters
 * @returns JSON string with simulation results
 */
export function simulateRace(paramsJson: string): string;

/**
 * Predict tire life and degradation
 * @param paramsJson - JSON string with tire life parameters
 * @returns JSON string with tire life prediction
 */
export function predictTireLife(paramsJson: string): string;
