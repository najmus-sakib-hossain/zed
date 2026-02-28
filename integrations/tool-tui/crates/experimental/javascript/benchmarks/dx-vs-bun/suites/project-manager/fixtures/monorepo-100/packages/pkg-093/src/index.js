/**
 * Package: pkg-093
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-092';
// import { something } from 'pkg-091';
// import { something } from 'pkg-090';

export function main() {
    console.log('pkg-093 main function');
    return { name: 'pkg-093', version: '1.0.0' };
}

export function helper93() {
    return 93 * 2;
}

export const config = {
    packageNumber: 93,
    totalPackages: 100
};
