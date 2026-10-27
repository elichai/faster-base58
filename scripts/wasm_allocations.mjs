import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const METRICS = [
    "alloc_calls", "alloc_bytes", "zeroed_calls", "zeroed_bytes",
    "realloc_calls", "realloc_old_bytes", "realloc_new_bytes", "dealloc_calls",
    "dealloc_bytes", "failed_calls", "live_before_bytes", "peak_live_bytes", "live_after_bytes",
];

export async function openCounted(path) {
    const { instance } = await WebAssembly.instantiate(await readFile(path), {});
    const api = instance.exports;
    assert(api.memory instanceof WebAssembly.Memory);
    assert(!(api.memory.buffer instanceof SharedArrayBuffer));
    assert.equal(api.size_count_metrics(), METRICS.length);
    const view = (pointer, length) => new Uint8Array(api.memory.buffer, pointer, length);
    const stats = () => {
        const result = Object.fromEntries(METRICS.map((name, index) => [name, api.size_count_get(index) >>> 0]));
        result.peak_extra_bytes = result.peak_live_bytes - result.live_before_bytes;
        result.retained_delta_bytes = result.live_after_bytes - result.live_before_bytes;
        assert.equal(result.failed_calls, 0);
        assert.equal(result.alloc_bytes + result.realloc_new_bytes - result.realloc_old_bytes - result.dealloc_bytes,
                     result.retained_delta_bytes);
        assert(result.peak_live_bytes >= Math.max(result.live_before_bytes, result.live_after_bytes));
        assert(result.zeroed_calls <= result.alloc_calls);
        return result;
    };
    return {
        api,
        selfCheck() {
            // Disposable instances only: this intentionally grows memory before any backend call.
            // Collect cold/page measurements on a new instance without this self-check.
            api.size_count_self_check();
            api.size_count_begin();
            const pointer = api.size_alloc(19) >>> 0;
            api.size_count_end();
            const allocated = stats();
            assert.equal(allocated.alloc_calls, 1);
            assert.equal(allocated.alloc_bytes, 19);
            assert.equal(allocated.retained_delta_bytes, 19);
            api.size_count_begin();
            api.size_dealloc(pointer, 19);
            api.size_count_end();
            const released = stats();
            assert.equal(released.alloc_calls, 0);
            assert.equal(released.dealloc_calls, 1);
            assert.equal(released.dealloc_bytes, 19);
            assert.equal(released.retained_delta_bytes, -19);
            const oldBuffer = api.memory.buffer;
            const length = oldBuffer.byteLength * 2 + 1;
            api.size_count_begin();
            const grown = api.size_alloc(length) >>> 0;
            api.size_count_end();
            assert.notEqual(api.memory.buffer, oldBuffer);
            assert(view(grown, length).every(byte => byte === 0));
            assert.equal(stats().alloc_bytes, length);
            api.size_dealloc(grown, length);
        },
        buffers(direction, input, capacity) {
            assert(input instanceof Uint8Array);
            assert(Number.isInteger(capacity) && capacity >= 0 && capacity < 0x80000000);
            const sourceLength = input.length + 34;
            const outputLength = capacity + 38;
            const sourceBase = api.size_alloc(sourceLength) >>> 0;
            const outputBase = api.size_alloc(outputLength) >>> 0;
            assert(sourceBase && outputBase);
            assert(sourceBase + sourceLength <= outputBase || outputBase + outputLength <= sourceBase);
            view(sourceBase, sourceLength).fill(0xA5);
            view(sourceBase + 17, input.length).set(input);
            return {
                call() {
                    view(outputBase, outputLength).fill(0xA5);
                    api.size_count_begin();
                    let written;
                    try {
                        written = api[`size_${direction}`](sourceBase + 17, input.length, outputBase + 19, capacity) >>> 0;
                    } finally {
                        api.size_count_end();
                    }
                    const measured = stats();
                    assert(view(sourceBase, 17).every(byte => byte === 0xA5));
                    assert.deepEqual(view(sourceBase + 17, input.length), input);
                    assert(view(sourceBase + 17 + input.length, 17).every(byte => byte === 0xA5));
                    assert(view(outputBase, 19).every(byte => byte === 0xA5));
                    assert(view(outputBase + 19 + capacity, 19).every(byte => byte === 0xA5));
                    assert(written === 0xffffffff || written <= capacity);
                    if (written !== 0xffffffff) assert(view(outputBase + 19 + written, capacity - written).every(byte => byte === 0xA5));
                    return { written, output: written === 0xffffffff ? null : view(outputBase + 19, written).slice(),
                        statistics: measured, linear_memory_bytes: api.memory.buffer.byteLength,
                        input_allocation_bytes: sourceLength, output_allocation_bytes: outputLength,
                        output_capacity_bytes: capacity };
                },
                close() {
                    api.size_dealloc(outputBase, outputLength);
                    api.size_dealloc(sourceBase, sourceLength);
                },
            };
        },
    };
}

function reference(bytes) {
    const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let value = 0n;
    for (const byte of bytes) value = (value << 8n) | BigInt(byte);
    let text = "";
    while (value) {
        text = alphabet[Number(value % 58n)] + text;
        value /= 58n;
    }
    let zeros = 0;
    while (zeros < bytes.length && bytes[zeros] === 0) zeros++;
    return new TextEncoder().encode("1".repeat(zeros) + text);
}

function zeroOperations(stats) {
    for (const name of METRICS.slice(0, 10)) assert.equal(stats[name], 0, name);
    assert.equal(stats.peak_extra_bytes, 0);
    assert.equal(stats.retained_delta_bytes, 0);
}

async function selfTest(directory) {
    for (const backend of ["empty", "auto", "no-alloc"]) {
        const path = join(directory, `${backend}-both-cdylib.wasm.stripped`);
        const probe = await openCounted(path);
        probe.selfCheck();
        for (const data of [new Uint8Array(), Uint8Array.of(0, 1), new Uint8Array(32),
                            Uint8Array.from({ length: 64 }, (_, i) => (i * 73 + 19) & 255)]) {
            const text = reference(data);
            for (const [operation, input, expected] of [["encode", data, text], ["decode", text, data]]) {
                const capacity = backend === "empty" ? Math.max(input.length, expected.length) : expected.length;
                const buffers = probe.buffers(operation, input, capacity);
                try {
                    for (const _phase of ["cold", "warm"]) {
                        const value = buffers.call();
                        assert.equal(value.written, backend === "empty" ? input.length : expected.length);
                        if (backend !== "empty") assert.deepEqual(value.output, expected);
                        zeroOperations(value.statistics);
                    }
                } finally { buffers.close(); }
            }
        }
        if (backend !== "empty") {
            for (const input of [Uint8Array.of(255), new TextEncoder().encode("0"), new TextEncoder().encode("é")]) {
                const buffers = probe.buffers("decode", input, input.length);
                try {
                    const value = buffers.call();
                    assert.equal(value.written, 0xffffffff);
                    zeroOperations(value.statistics);
                } finally { buffers.close(); }
            }
        }
        if (backend === "auto") {
            // Exercise allocating library calls as well as the direct allocator export.
            const positive = await openCounted(path);
            const data = Uint8Array.from({ length: 8193 }, (_, i) => (i * 73 + 19) & 255);
            const text = reference(data);
            for (const [operation, input, expected] of [["encode", data, text], ["decode", text, data]]) {
                const buffers = positive.buffers(operation, input, expected.length);
                try {
                    for (const _phase of ["cold", "warm"]) {
                        const value = buffers.call();
                        assert.deepEqual(value.output, expected);
                        assert(value.statistics.alloc_calls > 0, "allocating backend was not counted");
                        assert(value.statistics.peak_extra_bytes > 0);
                    }
                } finally { buffers.close(); }
            }
        }
        const invalid = await openCounted(path);
        assert.throws(() => invalid.api.size_count_get(METRICS.length), WebAssembly.RuntimeError);
        console.log(`Counted Wasm allocator/ABI self-check passed: ${backend}`);
    }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    assert.equal(process.argv[2], "--self-test");
    assert.equal(process.argv.length, 4);
    await selfTest(process.argv[3]);
}
