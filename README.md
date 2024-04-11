# Cloudflare fork of Zlib

This is a fork of zlib with performance improvements developed for use at Cloudflare.

This implementation only supports x86-64 with SSE 4.2+ and aarch64 with NEON & CRC. 32-bit CPUs are not supported at all.

The API and ABI are compatible with the original zlib.

This library is also available as a [Rust crate](https://lib.rs/crates/cloudflare-zlib).
