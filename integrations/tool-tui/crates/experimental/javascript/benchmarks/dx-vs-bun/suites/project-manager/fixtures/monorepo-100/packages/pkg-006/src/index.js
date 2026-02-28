/**
 * Package: pkg-006
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-005';
// import { something } from 'pkg-004';
// import { something } from 'pkg-003';

export function main() {
    console.log('pkg-006 main function');
    return { name: 'pkg-006', version: '1.0.0' };
}

export function helper6() {
    return 6 * 2;
}

export const config = {
    packageNumber: 6,
    totalPackages: 100
};
