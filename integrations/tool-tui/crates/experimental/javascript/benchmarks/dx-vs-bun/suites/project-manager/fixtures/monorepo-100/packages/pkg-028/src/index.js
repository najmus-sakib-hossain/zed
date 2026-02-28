/**
 * Package: pkg-028
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-027';
// import { something } from 'pkg-026';
// import { something } from 'pkg-025';

export function main() {
    console.log('pkg-028 main function');
    return { name: 'pkg-028', version: '1.0.0' };
}

export function helper28() {
    return 28 * 2;
}

export const config = {
    packageNumber: 28,
    totalPackages: 100
};
