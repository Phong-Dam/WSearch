/**
 * Context menu module
 */
import { CONTEXT_MENU_ITEMS } from './config.js';
import { state } from './state.js';
import { renderer } from './renderer.js';
import { showToast } from './toast.js';

/**
 * Context menu manager
 */
class ContextMenuManager {
    constructor() {
        this._menuElement = null;
        this._target = null;
    }

    /**
     * Initialize context menu
     * @param {HTMLElement} listElement - Results list element
     */
    init(listElement) {
        this._listElement = listElement;
        this._menuElement = document.getElementById('contextMenu');
        
        if (!this._menuElement) return;

        // Prevent default context menu
        listElement.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            
            const row = e.target.closest('.row');
            if (!row) return;

            // Use getAttribute to get raw value (no HTML entity decoding)
            const path = row.getAttribute('data-path');
            const name = row.getAttribute('data-name');
            const index = parseInt(row.getAttribute('data-index'));
            
            this._show(e.pageX, e.pageY, {
                path,
                name,
                index
            });
        });

        // Hide on click outside
        document.addEventListener('click', () => this._hide());

        // Hide on scroll
        listElement.addEventListener('scroll', () => this._hide());

        // Setup menu items
        this._setupMenuItems();
    }

    /**
     * Setup context menu item handlers
     * @private
     */
    _setupMenuItems() {
        document.getElementById('ctxOpen')?.addEventListener('click', () => {
            this._openFile();
        });

        document.getElementById('ctxShowFolder')?.addEventListener('click', () => {
            this._showInFolder();
        });

        document.getElementById('ctxCopyPath')?.addEventListener('click', () => {
            this._copyPath();
        });

        document.getElementById('ctxCopyName')?.addEventListener('click', () => {
            this._copyName();
        });
    }

    /**
     * Show context menu at position
     * @param {number} x - X coordinate
     * @param {number} y - Y coordinate
     * @param {Object} target - Target file info
     * @private
     */
    _show(x, y, target) {
        this._target = target;
        
        // Adjust position to stay within viewport
        const menuRect = this._menuElement.getBoundingClientRect();
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        let adjustedX = x;
        let adjustedY = y;

        if (x + menuRect.width > viewportWidth) {
            adjustedX = viewportWidth - menuRect.width - 10;
        }

        if (y + menuRect.height > viewportHeight) {
            adjustedY = viewportHeight - menuRect.height - 10;
        }

        this._menuElement.style.display = 'block';
        this._menuElement.style.left = adjustedX + 'px';
        this._menuElement.style.top = adjustedY + 'px';
    }

    /**
     * Hide context menu
     * @private
     */
    _hide() {
        if (this._menuElement) {
            this._menuElement.style.display = 'none';
        }
        this._target = null;
    }

    /**
     * Open file
     * @private
     */
    async _openFile() {
        if (!this._target) return;

        if (!window.__TAURI__) {
            showToast('Lỗi: Tauri không khả dụng', 'error');
            return;
        }

        try {
            await window.__TAURI__.core.invoke('open_path', { path: this._target.path });
            await window.__TAURI__.core.invoke('record_open', { path: this._target.path });
            
            // Select and open
            state.set({ selectedIndex: this._target.index });
            renderer.reset();
            renderer.render(
                state.get('results'),
                document.getElementById('results')?.scrollTop || 0,
                state.get('query'),
                this._target.index
            );
        } catch (error) {
            console.error('Failed to open file:', error);
            const errorMsg = error?.message || error?.toString() || String(error);
            showToast('Lỗi mở file: ' + errorMsg.substring(0, 60), 'error');
        }
        
        this._hide();
    }

    /**
     * Show file in folder
     * @private
     */
    async _showInFolder() {
        if (!this._target) return;

        try {
            await window.__TAURI__.core.invoke('show_in_folder', { path: this._target.path });
        } catch (error) {
            console.error('Failed to show in folder:', error);
            showToast('Failed to show in folder', 'error');
        }
        
        this._hide();
    }

    /**
     * Copy file path to clipboard
     * @private
     */
    async _copyPath() {
        if (!this._target) return;

        try {
            await navigator.clipboard.writeText(this._target.path);
            showToast('Path copied to clipboard');
        } catch (error) {
            console.error('Failed to copy path:', error);
            showToast('Failed to copy path', 'error');
        }
        
        this._hide();
    }

    /**
     * Copy file name to clipboard
     * @private
     */
    async _copyName() {
        if (!this._target) return;

        try {
            await navigator.clipboard.writeText(this._target.name);
            showToast('Name copied to clipboard');
        } catch (error) {
            console.error('Failed to copy name:', error);
            showToast('Failed to copy name', 'error');
        }
        
        this._hide();
    }
}

// Export singleton
export const contextMenu = new ContextMenuManager();
