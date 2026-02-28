/**
 * Package: pkg-083
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-082';
// import { something } from 'pkg-081';
// import { something } from 'pkg-080';

export function main() {
    console.log('pkg-083 main function');
    return { name: 'pkg-083', version: '1.0.0' };
}

export function helper83() {
    return 83 * 2;
}

export const config = {
    packageNumber: 83,
    totalPackages: 100
};
