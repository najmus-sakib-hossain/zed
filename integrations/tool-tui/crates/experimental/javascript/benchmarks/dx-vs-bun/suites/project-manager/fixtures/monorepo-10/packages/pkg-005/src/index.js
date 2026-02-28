/**
 * Package: pkg-005
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-004';
// import { something } from 'pkg-003';
// import { something } from 'pkg-002';

export function main() {
    console.log('pkg-005 main function');
    return { name: 'pkg-005', version: '1.0.0' };
}

export function helper5() {
    return 5 * 2;
}

export const config = {
    packageNumber: 5,
    totalPackages: 10
};
