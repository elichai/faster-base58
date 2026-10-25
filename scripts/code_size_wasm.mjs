import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const UTF8 = new TextDecoder("utf-8", { fatal: true });

// Allocation/copying is outside the measured repeated-call interval.
export async function openProbe(path) {
    const { instance } = await WebAssembly.instantiate(await readFile(path), {});
    const api = instance.exports;
    assert(api.memory instanceof WebAssembly.Memory);
    const decodePrevalidated = typeof api.size_repeat_decode_prevalidated === "function";
    if (decodePrevalidated) assert(!(api.memory.buffer instanceof SharedArrayBuffer), "prevalidated decoding requires unshared Wasm memory");
    return {
        exports: Object.keys(api),
        decodePrevalidated,
        checkAllocator() {
            for (const length of [0, 1, 32, 65537]) {
                const pointer = api.size_alloc(length) >>> 0;
                assert.notEqual(pointer, 0);
                const storage = new Uint8Array(api.memory.buffer, pointer, length);
                assert(storage.every(byte => byte === 0));
                storage.fill(0xA5);
                api.size_dealloc(pointer, length);
            }
        },
        callChecked(direction, input, capacity) {
            return this.call(direction, input, 1, capacity, true);
        },
        call(direction, input, iterations = 1, capacity = input.length * 2 + 32, checked = false) {
            assert(input instanceof Uint8Array);
            assert(input.length < 0x80000000);
            assert(Number.isInteger(iterations) && iterations >= 0 && iterations < 0x80000000);
            const prevalidated = direction === "decode" && decodePrevalidated && !checked;
            const operation = api[checked ? `size_${direction}` : prevalidated
                ? "size_repeat_decode_prevalidated" : `size_repeat_${direction}`];
            assert.equal(typeof operation, "function");
            assert(Number.isInteger(capacity) && capacity >= 0 && capacity < 0x80000000);
            const source = api.size_alloc(input.length) >>> 0;
            const output = api.size_alloc(capacity) >>> 0;
            try {
                assert(source !== 0 && output !== 0);
                assert(source + input.length <= output || output + capacity <= source, "input/output allocations overlap");
                const storage = new Uint8Array(api.memory.buffer, source, input.length);
                storage.set(input);
                // Validate the actual unshared Wasm input, then keep it unchanged through the call.
                if (prevalidated) UTF8.decode(storage);
                const start = process.hrtime.bigint();
                const written = operation(source, input.length, output, capacity, iterations) >>> 0;
                const elapsedNs = Number(process.hrtime.bigint() - start);
                assert(written === 0xffffffff || written <= capacity);
                return {
                    written, elapsedNs,
                    output: written === 0xffffffff ? null : new Uint8Array(api.memory.buffer, output, written).slice(),
                    memoryBytes: api.memory.buffer.byteLength,
                };
            } finally {
                api.size_dealloc(output, capacity);
                api.size_dealloc(source, input.length);
            }
        },
    };
}

async function main() {
    const paths = process.argv.slice(2);
    assert(paths.length, "pass one or more Wasm modules for the ABI self-check");
    for (const path of paths) {
        const probe = await openProbe(path);
        probe.checkAllocator();
        const empty = /(?:^|\/)empty-/.test(path);
        const five8 = /(?:^|\/)five8-/.test(path);
        const data = five8 ? new Uint8Array(32) : Uint8Array.of(0, 1);
        const text = new TextEncoder().encode(five8 ? "1".repeat(32) : "12");
        if (probe.exports.includes("size_encode")) {
            const value = probe.call("encode", data, 3);
            assert.equal(value.written, empty ? data.length : text.length);
            if (!empty) assert.deepEqual(value.output, text);
            assert.equal(probe.call("encode", data, 0).written, 0);
            if (!empty) assert.equal(probe.callChecked("encode", data, 0).written, 0xffffffff);
        }
        if (probe.exports.includes("size_decode")) {
            const capacity = five8 ? data.length : undefined;
            const value = probe.call("decode", text, 3, capacity);
            assert.equal(value.written, empty ? text.length : data.length);
            if (!empty) {
                assert.deepEqual(value.output, data);
                assert.equal(probe.callChecked("decode", Uint8Array.of(255), capacity).written, 0xffffffff);
                if (probe.decodePrevalidated) assert.throws(() => probe.call("decode", Uint8Array.of(255), 1, capacity), TypeError);
                assert.equal(probe.call("decode", new TextEncoder().encode("0"), 1, capacity).written, 0xffffffff);
                assert.equal(probe.callChecked("decode", text, 0).written, 0xffffffff);
                assert.equal(probe.call("decode", new TextEncoder().encode("é"), 1, capacity).written, 0xffffffff);
            }
            assert.equal(probe.call("decode", text, 0, capacity).written, 0);
        }
        console.log(`Wasm ABI self-check passed: ${path}`);
    }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
