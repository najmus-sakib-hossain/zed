/**
 * Package: pkg-022
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-021';
// import { something } from 'pkg-020';
// import { something } from 'pkg-019';

export function main() {
    console.log('pkg-022 main function');
    return { name: 'pkg-022', version: '1.0.0' };
}

export function helper22() {
    return 22 * 2;
}

export const config = {
    packageNumber: 22,
    totalPackages: 100
};
