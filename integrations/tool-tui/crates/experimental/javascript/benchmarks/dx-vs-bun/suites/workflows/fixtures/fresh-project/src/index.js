/**
 * Fresh Project - Main Entry Point
 * Used for workflow benchmarks
 */

const _ = require('lodash');

function main() {
    const data = [1, 2, 3, 4, 5];
    const doubled = _.map(data, n => n * 2);
    console.log('Doubled:', doubled);
    return doubled;
}

module.exports = { main };

if (require.main === module) {
    main();
}
