/**
 * Utility functions for dev iteration project
 */

function processData(data) {
    return data.map(item => ({
        ...item,
        processed: true,
        timestamp: Date.now()
    }));
}

function validateInput(input) {
    if (!input || typeof input !== 'object') {
        throw new Error('Invalid input');
    }
    return true;
}

module.exports = { processData, validateInput };
