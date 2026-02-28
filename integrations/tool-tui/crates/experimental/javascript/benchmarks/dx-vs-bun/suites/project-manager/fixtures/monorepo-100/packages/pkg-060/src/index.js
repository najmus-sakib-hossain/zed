/**
 * Package: pkg-060
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-059';
// import { something } from 'pkg-058';
// import { something } from 'pkg-057';

export function main() {
    console.log('pkg-060 main function');
    return { name: 'pkg-060', version: '1.0.0' };
}

export function helper60() {
    return 60 * 2;
}

export const config = {
    packageNumber: 60,
    totalPackages: 100
};
