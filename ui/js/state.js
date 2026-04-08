/**
 * State management module
 * Centralized state with reactive updates
 */
import { DEFAULT_STATE } from './config.js';

/**
 * State manager class
 */
class StateManager {
    constructor() {
        this._state = { ...DEFAULT_STATE };
        this._listeners = new Map();
    }

    /**
     * Get current state or specific key
     * @param {string} [key] - Optional key to get
     * @returns {*} State value or entire state object
     */
    get(key) {
        if (key) {
            return this._state[key];
        }
        return { ...this._state };
    }

    /**
     * Set one or more state values
     * @param {Object|string} keyOrUpdate - Object with updates or string key
     * @param {*} [value] - Value if key is string
     * @returns {Object} Previous values that changed
     */
    set(keyOrUpdate, value) {
        const changes = {};
        
        if (typeof keyOrUpdate === 'object') {
            Object.entries(keyOrUpdate).forEach(([key, val]) => {
                if (this._state[key] !== val) {
                    changes[key] = this._state[key];
                    this._state[key] = val;
                }
            });
        } else {
            if (this._state[keyOrUpdate] !== value) {
                changes[keyOrUpdate] = this._state[keyOrUpdate];
                this._state[keyOrUpdate] = value;
            }
        }

        // Notify listeners of changes
        if (Object.keys(changes).length > 0) {
            this._notifyListeners(changes);
        }

        return changes;
    }

    /**
     * Subscribe to state changes
     * @param {string|string[]} keys - Keys to watch (or '*' for all)
     * @param {Function} callback - Callback function(changes)
     * @returns {Function} Unsubscribe function
     */
    subscribe(keys, callback) {
        const keyArray = Array.isArray(keys) ? keys : [keys];
        const id = Symbol('listener');
        
        this._listeners.set(id, { keys: keyArray, callback });
        
        return () => this._listeners.delete(id);
    }

    /**
     * Notify listeners of state changes
     * @param {Object} changes - Changed values
     */
    _notifyListeners(changes) {
        this._listeners.forEach(({ keys, callback }) => {
            if (keys.includes('*')) {
                callback(changes);
            } else {
                const relevantChanges = {};
                keys.forEach(key => {
                    if (key in changes) {
                        relevantChanges[key] = changes[key];
                    }
                });
                if (Object.keys(relevantChanges).length > 0) {
                    callback(relevantChanges);
                }
            }
        });
    }
}

// Export singleton instance
export const state = new StateManager();
