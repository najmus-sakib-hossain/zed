/**
 * Package: pkg-097
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-096';
// import { something } from 'pkg-095';
// import { something } from 'pkg-094';

export function main() {
    console.log('pkg-097 main function');
    return { name: 'pkg-097', version: '1.0.0' };
}

export function helper97() {
    return 97 * 2;
}

export const config = {
    packageNumber: 97,
    totalPackages: 100
};
