/**
 * Toast notification module
 */

/**
 * Toast notification manager
 */
class ToastManager {
    constructor() {
        this._container = null;
        this._toasts = new Map();
        this._counter = 0;
    }

    /**
     * Initialize toast container
     */
    init() {
        // Create container if not exists
        if (!document.getElementById('toast-container')) {
            this._container = document.createElement('div');
            this._container.id = 'toast-container';
            this._container.className = 'toast-container';
            document.body.appendChild(this._container);
        } else {
            this._container = document.getElementById('toast-container');
        }
    }

    /**
     * Show a toast notification
     * @param {string} message - Message to display
     * @param {string} [type='info'] - Type: 'success', 'error', 'warning', 'info'
     * @param {number} [duration=3000] - Duration in milliseconds
     */
    show(message, type = 'info', duration = 3000) {
        if (!this._container) {
            this.init();
        }

        const id = ++this._counter;
        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.dataset.id = id;
        toast.textContent = message;

        this._container.appendChild(toast);
        this._toasts.set(id, toast);

        // Trigger animation
        requestAnimationFrame(() => {
            toast.classList.add('toast-visible');
        });

        // Auto remove
        setTimeout(() => {
            this._remove(id);
        }, duration);

        return id;
    }

    /**
     * Remove a toast
     * @param {number} id 
     * @private
     */
    _remove(id) {
        const toast = this._toasts.get(id);
        if (!toast) return;

        toast.classList.remove('toast-visible');
        toast.classList.add('toast-hidden');

        setTimeout(() => {
            toast.remove();
            this._toasts.delete(id);
        }, 300);
    }

    /**
     * Clear all toasts
     */
    clear() {
        this._toasts.forEach((toast, id) => {
            this._remove(id);
        });
    }
}

// Export singleton
export const toastManager = new ToastManager();

// Convenience export
export const showToast = (message, type, duration) => toastManager.show(message, type, duration);
