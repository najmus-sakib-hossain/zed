/**
 * Package: pkg-023
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-022';
// import { something } from 'pkg-021';
// import { something } from 'pkg-020';

export function main() {
    console.log('pkg-023 main function');
    return { name: 'pkg-023', version: '1.0.0' };
}

export function helper23() {
    return 23 * 2;
}

export const config = {
    packageNumber: 23,
    totalPackages: 100
};
