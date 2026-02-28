/**
 * Package: pkg-096
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-095';
// import { something } from 'pkg-094';
// import { something } from 'pkg-093';

export function main() {
    console.log('pkg-096 main function');
    return { name: 'pkg-096', version: '1.0.0' };
}

export function helper96() {
    return 96 * 2;
}

export const config = {
    packageNumber: 96,
    totalPackages: 100
};
