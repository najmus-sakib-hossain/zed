/**
 * Package: pkg-003
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-002';
// import { something } from 'pkg-001';

export function main() {
    console.log('pkg-003 main function');
    return { name: 'pkg-003', version: '1.0.0' };
}

export function helper3() {
    return 3 * 2;
}

export const config = {
    packageNumber: 3,
    totalPackages: 10
};
