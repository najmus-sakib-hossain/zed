/**
 * Package: pkg-002
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-001';

export function main() {
    console.log('pkg-002 main function');
    return { name: 'pkg-002', version: '1.0.0' };
}

export function helper2() {
    return 2 * 2;
}

export const config = {
    packageNumber: 2,
    totalPackages: 50
};
