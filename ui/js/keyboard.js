/**
 * Keyboard navigation module
 */
import { CONFIG } from './config.js';
import { state } from './state.js';
import { renderer } from './renderer.js';
import { sortResults } from './utils.js';

/**
 * Keyboard navigation handler
 */
class KeyboardHandler {
    /**
     * Initialize keyboard handling
     * @param {HTMLElement} listElement - Results list element
     */
    init(listElement) {
        this._listElement = listElement;
        
        document.addEventListener('keydown', this._handleKeyDown.bind(this));
    }

    /**
     * Handle keydown events
     * @param {KeyboardEvent} event
     * @private
     */
    _handleKeyDown(event) {
        const results = state.get('results');
        if (results.length === 0) return;

        switch (event.key) {
            case 'ArrowDown':
                this._selectNext();
                event.preventDefault();
                break;
                
            case 'ArrowUp':
                this._selectPrevious();
                event.preventDefault();
                break;
                
            case 'Enter':
                this._openSelected();
                event.preventDefault();
                break;
                
            case 'Escape':
                this._clearSelection();
                event.preventDefault();
                break;
        }
    }

    /**
     * Select next item
     */
    _selectNext() {
        const results = state.get('results');
        const selectedIndex = state.get('selectedIndex');
        const newIndex = Math.min(selectedIndex + 1, results.length - 1);
        
        state.set({ selectedIndex: newIndex });
        renderer.reset();
        
        this._ensureVisible();
        this._updateSelection();
    }

    /**
     * Select previous item
     */
    _selectPrevious() {
        const selectedIndex = state.get('selectedIndex');
        const newIndex = Math.max(selectedIndex - 1, 0);
        
        state.set({ selectedIndex: newIndex });
        renderer.reset();
        
        this._ensureVisible();
        this._updateSelection();
    }

    /**
     * Ensure selected item is visible in viewport
     * @private
     */
    _ensureVisible() {
        if (!this._listElement) return;

        const selectedIndex = state.get('selectedIndex');
        const rowY = selectedIndex * CONFIG.ROW_HEIGHT;
        const viewportTop = this._listElement.scrollTop;
        const viewportHeight = this._listElement.clientHeight;

        // Scroll up if needed
        if (rowY < viewportTop) {
            this._listElement.scrollTop = rowY;
        }
        
        // Scroll down if needed
        if (rowY > viewportTop + viewportHeight - CONFIG.ROW_HEIGHT) {
            this._listElement.scrollTop = rowY - viewportHeight + CONFIG.ROW_HEIGHT;
        }
    }

    /**
     * Update visual selection
     * @private
     */
    _updateSelection() {
        if (!this._listElement) return;

        const selectedIndex = state.get('selectedIndex');
        const scrollTop = this._listElement.scrollTop;
        const query = state.get('query');
        const results = state.get('results');

        renderer.render(results, scrollTop, query, selectedIndex);
    }

    /**
     * Open selected item
     */
    async _openSelected() {
        const results = state.get('results');
        const selectedIndex = state.get('selectedIndex');
        const file = results[selectedIndex];

        if (!file?.path) return;

        try {
            await window.__TAURI__.core.invoke('open_path', { path: file.path });
            await window.__TAURI__.core.invoke('record_open', { path: file.path });
        } catch (error) {
            console.error('Failed to open file:', error);
        }
    }

    /**
     * Clear selection and input
     */
    _clearSelection() {
        state.set({
            results: [],
            selectedIndex: 0,
            query: ''
        });
        
        renderer.reset();
        renderer.render([], 0, '', 0);
        
        const input = document.getElementById('search');
        if (input) {
            input.value = '';
            input.focus();
        }
    }
}

/**
 * Sort handler for column headers
 */
class SortHandler {
    constructor() {
        this._init();
    }

    /**
     * Initialize sort handling
     * @private
     */
    _init() {
        const header = document.getElementById('header');
        if (!header) return;

        header.addEventListener('click', (e) => {
            const field = e.target.dataset.sort;
            if (!field) return;

            const currentField = state.get('sortField');
            const currentDir = state.get('sortDirection');

            if (currentField === field) {
                // Toggle direction
                state.set({ sortDirection: currentDir * -1 });
            } else {
                // New field, ascending
                state.set({ sortField: field, sortDirection: 1 });
            }

            // Re-sort and render
            this._resortResults();
        });
    }

    /**
     * Re-sort results with new sort settings
     * @private
     */
    _resortResults() {
        const results = state.get('results');
        const sorted = sortResults(
            results,
            state.get('sortField'),
            state.get('sortDirection')
        );

        state.set({ results: sorted, selectedIndex: 0 });
        
        const listElement = document.getElementById('results');
        if (listElement) {
            renderer.reset();
            renderer.render(sorted, listElement.scrollTop, state.get('query'), 0);
        }
    }
}

// Export singleton instances
export const keyboardHandler = new KeyboardHandler();
export const sortHandler = new SortHandler();
