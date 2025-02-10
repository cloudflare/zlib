use std::env;
use std::path::PathBuf;

fn main() {
    let mut cc = cc::Build::new();
    let comp = cc.get_compiler();
    let is_msvc = comp.is_like_msvc();
    cc.warnings(false);
    cc.pic(true);

    let target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").expect("CARGO_CFG_TARGET_POINTER_WIDTH");
    let target_endian = env::var("CARGO_CFG_TARGET_ENDIAN").expect("CARGO_CFG_TARGET_ENDIAN");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH");
    let target_feature = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let has_feature = |f| target_feature.split(',').any(|t| t == f);

    if target_endian != "little" {
        println!("cargo::error=cloudflare-zlib does not support big endian");
        // don't exit(1): https://github.com/rust-lang/cargo/issues/15038
    }

    if target_pointer_width != "64" {
        println!("cargo:warning=cloudflare-zlib does not support 32-bit architectures");
        println!("cargo::error=cloudflare-zlib does not support 32-bit architectures");
        // don't exit(1): https://github.com/rust-lang/cargo/issues/15038
    }

    if let Ok(target_cpu) = env::var("TARGET_CPU") {
        cc.flag_if_supported(&format!("-march={}", target_cpu));
    }

    let vendor = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:include={}", vendor.display());
    cc.include(&vendor);

    if target_arch == "aarch64" {
        cc.define("INFLATE_CHUNK_SIMD_NEON", Some("1"));
        cc.define("ADLER32_SIMD_NEON", Some("1"));
        cc.flag_if_supported(if is_msvc { "/arch:armv8.3" } else { "-march=armv8-a+crc" });
    } else if target_arch == "x86_64" {
        let sse42_flag = if is_msvc { "/arch:SSE4.2" } else { "-msse4.2" };

        let has_avx = has_feature("avx"); // AVX changes ABI, so it can't be enabled just in C
        let has_sse42 = has_avx || has_feature("sse4.2") ||
            cc.is_flag_supported(sse42_flag).unwrap_or(false) ||
            // MSDN says /arch:SSE4.2 exists, but MSVC disagrees.
            (is_msvc && cc.is_flag_supported("/arch:AVX").unwrap_or(false));
        let has_ssse3 = has_avx || has_feature("ssse3") || cc.is_flag_supported("-mssse3").unwrap_or(false);
        let has_pcmul = has_feature("pclmulqdq") || cc.is_flag_supported("-mpclmul").unwrap_or(false);

        if has_avx {
            cc.define("HAS_AVX", Some("1"));
            cc.flag(if is_msvc { "/arch:AVX" } else { "-mavx" });
        } else if has_sse42 {
            cc.flag(sse42_flag);
        }

        if has_sse42 {
            cc.define("INFLATE_CHUNK_SIMD_SSE2", Some("1"));
            if is_msvc {
                cc.define("__x86_64__", Some("1"));
            }
            cc.define("HAS_SSE2", Some("1"));
            cc.define("HAS_SSE42", Some("1"));

            if has_ssse3 {
                cc.flag_if_supported("-mssse3");
                cc.define("HAS_SSSE3", Some("1"));
                cc.define("ADLER32_SIMD_SSSE3", Some("1"));
            }

            if has_pcmul {
                cc.flag_if_supported("-mpclmul");
                cc.define("HAS_PCLMUL", Some("1"));
                cc.file(vendor.join("crc32_simd.c"));

                if has_feature("pclmulqdq") {
                    cc.define("SKIP_CPUID_CHECK", Some("1"));
                }
            }

        } else {
            println!("cargo::error=This build has disabled SSE4.2 support, but cloudflare-zlib-sys requires it");
            println!("cargo:warning=Build with RUSTFLAGS='-Ctarget-feature=+sse4.2. Currently enabled: {target_feature}");
            cc.define("INFLATE_CHUNK_GENERIC", Some("1"));
        }
    }

    cc.define("INFLATE_CHUNK_READ_64LE", Some("1"));

    if is_msvc {
        cc.define("_CRT_SECURE_NO_DEPRECATE", Some("1"));
        cc.define("_CRT_NONSTDC_NO_DEPRECATE", Some("1"));
    } else {
        cc.define("HAVE_UNISTD_H", Some("1"));
        cc.define("HAVE_HIDDEN", Some("1"));

        if "32" != target_pointer_width {
            cc.define("HAVE_OFF64_T", Some("1"));
            cc.define("_LARGEFILE64_SOURCE", Some("1"));
        }
    }

    cc.file(vendor.join("adler32.c"));
    cc.file(vendor.join("adler32_simd.c"));
    cc.file(vendor.join("crc32.c"));
    cc.file(vendor.join("deflate.c"));
    cc.file(vendor.join("infback.c"));
    cc.file(vendor.join("inffast.c"));
    cc.file(vendor.join("inflate.c"));
    cc.file(vendor.join("inftrees.c"));
    cc.file(vendor.join("trees.c"));
    cc.file(vendor.join("zutil.c"));

    cc.file(vendor.join("compress.c"));
    cc.file(vendor.join("uncompr.c"));
    cc.file(vendor.join("gzclose.c"));
    cc.file(vendor.join("gzlib.c"));
    cc.file(vendor.join("gzread.c"));
    cc.file(vendor.join("gzwrite.c"));

    cc.file(vendor.join("inffast_chunk.c"));

    cc.compile("cloudflare_zlib");
}
