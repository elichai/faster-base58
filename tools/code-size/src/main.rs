#![allow(unexpected_cfgs)]

mod entry;

fn main() {
    let arg = std::env::args_os().nth(1).unwrap_or_default();
    let input = arg.as_encoded_bytes();
    let mut output = vec![0; input.len().saturating_mul(2).saturating_add(1)];
    #[cfg(not(size_direction = "decode"))]
    let written = unsafe {
        entry::size_encode(input.as_ptr(), input.len(), output.as_mut_ptr(), output.len())
    };
    #[cfg(size_direction = "decode")]
    let written = unsafe {
        entry::size_decode(input.as_ptr(), input.len(), output.as_mut_ptr(), output.len())
    };
    #[cfg(size_direction = "both")]
    {
        let mut decoded = vec![0; input.len()];
        let read = unsafe {
            entry::size_decode(output.as_ptr(), written.min(output.len()), decoded.as_mut_ptr(), decoded.len())
        };
        std::hint::black_box((read, decoded));
    }
    std::hint::black_box((written, output));
}
