/**
 * Package: pkg-004
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-003';
// import { something } from 'pkg-002';
// import { something } from 'pkg-001';

export function main() {
    console.log('pkg-004 main function');
    return { name: 'pkg-004', version: '1.0.0' };
}

export function helper4() {
    return 4 * 2;
}

export const config = {
    packageNumber: 4,
    totalPackages: 100
};
