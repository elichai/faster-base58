import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { openCounted } from '../../../../scripts/wasm_allocations.mjs';

const [directory, fixtureDirectory, manifestPath, backend, operation, sizeText, moduleHash] = process.argv.slice(2);
assert.equal(process.argv.length, 9);
assert(['encode', 'decode'].includes(operation));
const size = Number(sizeText);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
const fixture = manifest.fixtures.find(row => row.decoded_bytes === size);
assert(fixture);
const data = new Uint8Array(await readFile(join(fixtureDirectory, `${size}.bin`)));
const text = new Uint8Array(await readFile(join(fixtureDirectory, `${size}.txt`)));
assert.equal(hash(data), fixture.data_sha256);
assert.equal(hash(text), fixture.encoded_sha256);
const modulePath = join(directory, `${backend}-both-cdylib.wasm.stripped`);
assert.equal(hash(await readFile(modulePath)), moduleHash);
// This instance has no selfCheck/growth call or earlier cache history.
const probe = await openCounted(modulePath);
const [input, expected] = operation === 'encode' ? [data, text] : [text, data];
const capacity = backend === 'empty' ? Math.max(input.length, expected.length) : expected.length;
const pagesBefore = probe.api.memory.buffer.byteLength / 65536;
const buffers = probe.buffers(operation, input, capacity);
const rows = [];
try {
    for (const phase of ['cold', 'warm']) {
        const value = buffers.call();
        assert.equal(value.written, backend === 'empty' ? input.length : expected.length);
        if (backend !== 'empty') assert.deepEqual(value.output, expected);
        if (['empty', 'no-alloc', 'bs58', 'five8'].includes(backend)) {
            for (const [metric, count] of Object.entries(value.statistics)) {
                if (!['live_before_bytes', 'peak_live_bytes', 'live_after_bytes'].includes(metric)) {
                    assert.equal(count, 0, `${backend}/${operation}/${size}/${phase}/${metric}`);
                }
            }
        }
        rows.push({ phase, ...value.statistics, linear_memory_bytes: value.linear_memory_bytes,
            input_allocation_bytes: value.input_allocation_bytes, output_allocation_bytes: value.output_allocation_bytes,
            output_capacity_bytes: value.output_capacity_bytes });
    }
} finally { buffers.close(); }
assert.equal(hash(await readFile(modulePath)), moduleHash);
process.stdout.write(JSON.stringify({ backend, operation, decoded_bytes: size, input_bytes: input.length,
    encoded_bytes: text.length, fixture, module_sha256: moduleHash, initial_linear_memory_pages: pagesBefore, rows }));
