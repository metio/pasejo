// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Sandbox the QR-code decoding step.
//!
//! `image` plus `rqrr` are the largest attacker-controlled-input parser
//! stack in pasejo: a user can be talked into running
//! `pasejo otp add --qrcode bad.png …` against a maliciously crafted
//! image, and a memory-corruption bug in either crate would otherwise
//! give the attacker pasejo's full filesystem reach — SSH keys, age
//! identity files, the encrypted store, anything the user can read.
//!
//! On Linux the decode runs in a `fork()`ed child that immediately
//! installs a Landlock ruleset denying every filesystem access right the
//! running kernel knows about. The child reads the image bytes off disk
//! *before* enforcing the ruleset, then runs `image::load_from_memory`
//! and `rqrr` purely on the in-memory buffer and writes the decoded
//! `otpauth://` URL (or a structured error message) back to the parent
//! over a `pipe(2)`. The parent waits for the child and feeds the URL
//! into the normal OTP parser.
//!
//! Landlock is permanent for the calling process once `restrict_self`
//! returns, which is why the work has to happen across a `fork(2)` —
//! the parent must remain unrestricted so the surrounding
//! decrypt/encrypt/hooks pipeline can continue. On non-Linux targets
//! (and on Linux kernels too old for Landlock) the decode runs
//! in-process; the byte→URL path is unchanged, so behaviour is identical
//! from the caller's perspective and the sandbox is purely additive
//! defence in depth.

use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::i18n;

/// Read the QR code at `path` and return the embedded `otpauth://` URL.
///
/// On Linux the decoding runs in a forked child that has installed a
/// Landlock ruleset denying all filesystem access. On other platforms
/// the decoding runs in-process. Either way the public contract is the
/// same: returns the QR payload string on success, an
/// [`anyhow::Error`] carrying the localized "no QR code found" /
/// "failed to decode" message otherwise.
pub fn decode_qrcode_to_otpauth_url(path: &Path) -> Result<String> {
    #[cfg(target_os = "linux")]
    {
        linux::decode_in_sandboxed_child(path)
    }
    #[cfg(not(target_os = "linux"))]
    {
        decode_qrcode_from_file(path)
    }
}

/// In-process fallback decoder. Reads the file and runs
/// [`decode_qrcode_bytes`] on its contents. Used directly on non-Linux
/// targets; on Linux the production code path goes through the
/// sandboxed child, so this helper is reserved for tests of the
/// fallback semantics.
#[cfg(any(test, not(target_os = "linux")))]
fn decode_qrcode_from_file(path: &Path) -> Result<String> {
    let qrcode_display = path.display().to_string();
    let bytes =
        std::fs::read(path).with_context(|| i18n::error_cannot_read_file(&qrcode_display))?;
    decode_qrcode_bytes(&bytes, &qrcode_display)
}

/// Pure decoder: take raw image bytes already in memory and return the
/// embedded QR payload as a string. Format detection is left to the
/// `image` crate.
///
/// The split between "read bytes" and "decode bytes" lets the Linux
/// sandbox open the file before enforcing the ruleset, so the parser
/// itself never needs filesystem access.
pub fn decode_qrcode_bytes(bytes: &[u8], qrcode_display: &str) -> Result<String> {
    let img = image::load_from_memory(bytes)
        .with_context(|| i18n::error_failed_to_decode_qrcode(qrcode_display))?
        .to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(img);
    let grids = prepared.detect_grids();
    let grid = grids
        .first()
        .ok_or_else(|| anyhow::anyhow!(i18n::error_no_qrcode_found(qrcode_display)))?;
    let (_, content) = grid
        .decode()
        .with_context(|| i18n::error_failed_to_decode_qrcode(qrcode_display))?;
    Ok(content)
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::path::Path;

    use anyhow::{Context, Result, anyhow, bail};
    use landlock::{
        ABI, Access, AccessFs, CompatLevel, Compatible, Ruleset, RulesetAttr, RulesetStatus,
    };

    use super::decode_qrcode_bytes;
    use crate::cli::i18n;

    /// First byte of the pipe payload signalling a successful decode;
    /// the rest of the buffer is the decoded `otpauth://` URL bytes.
    const TAG_OK: u8 = 0;
    /// First byte of the pipe payload signalling an error in the
    /// child; the rest of the buffer is the rendered error message.
    const TAG_ERR: u8 = 1;

    /// Linux entry point. Sets up the pipe, forks, drives both ends.
    pub(super) fn decode_in_sandboxed_child(path: &Path) -> Result<String> {
        let (read_fd, write_fd) = create_pipe()?;

        // SAFETY: `libc::fork` is the documented Unix fork primitive.
        // pasejo is single-threaded at this point — `parse_password`
        // runs from `main` before any worker thread is spawned — so the
        // child does not inherit any thread state and is free to call
        // arbitrary code paths until it `_exit`s.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            let err = std::io::Error::last_os_error();
            drop(read_fd);
            drop(write_fd);
            return Err(err).context(i18n::error_qr_sandbox_failed());
        }

        if pid == 0 {
            drop(read_fd);
            child_main(path, write_fd);
            // child_main calls libc::_exit and does not return.
        }

        drop(write_fd);
        read_child_response(read_fd, pid)
    }

    /// Create a `pipe(2)` and wrap the raw file descriptors as
    /// [`OwnedFd`] so they get closed automatically on any return path.
    fn create_pipe() -> Result<(OwnedFd, OwnedFd)> {
        let mut fds = [0_i32; 2];
        // SAFETY: `libc::pipe` writes exactly two file descriptors into
        // the buffer we just allocated and returns 0 on success.
        let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if rc != 0 {
            return Err(std::io::Error::last_os_error()).context(i18n::error_qr_sandbox_failed());
        }
        // SAFETY: `pipe` returns two freshly-allocated file descriptors
        // that this process now owns; transferring ownership to
        // `OwnedFd` ensures they're closed on drop.
        let read_fd = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        // SAFETY: see above.
        let write_fd = unsafe { OwnedFd::from_raw_fd(fds[1]) };
        Ok((read_fd, write_fd))
    }

    /// Drain the response pipe, reap the child with `waitpid`, and
    /// translate the tagged response back into an `anyhow::Result`.
    fn read_child_response(read_fd: OwnedFd, pid: libc::pid_t) -> Result<String> {
        let mut buf = Vec::new();
        let read_result = File::from(read_fd).read_to_end(&mut buf);

        let mut status = 0_i32;
        // SAFETY: `pid` is the child we just forked; we are its parent
        // and have not yet reaped it.
        let waited = unsafe { libc::waitpid(pid, &raw mut status, 0) };
        if waited < 0 {
            return Err(std::io::Error::last_os_error()).context(i18n::error_qr_sandbox_failed());
        }

        read_result.context(i18n::error_qr_sandbox_failed())?;
        decode_response(&buf, status)
    }

    /// Pure helper: turn the raw pipe payload plus waitpid status into a
    /// success URL or a structured error. Factored out so it can be
    /// unit-tested without forking.
    fn decode_response(buf: &[u8], status: i32) -> Result<String> {
        let Some((&tag, rest)) = buf.split_first() else {
            // The child died before writing anything — almost certainly
            // a fatal signal (segfault, SIGKILL by OOM, etc.). Surface
            // that explicitly so it isn't confused with a parse error.
            if libc_wifsignaled(status) {
                bail!(i18n::error_qr_sandbox_child_signal(
                    &libc_wtermsig(status).to_string()
                ));
            }
            bail!(i18n::error_qr_sandbox_failed());
        };

        let payload = std::str::from_utf8(rest)
            .map(str::to_owned)
            .context(i18n::error_qr_sandbox_failed())?;

        if tag == TAG_OK {
            Ok(payload)
        } else if tag == TAG_ERR {
            Err(anyhow!(payload))
        } else {
            Err(anyhow!(i18n::error_qr_sandbox_failed()))
        }
    }

    /// Child entry point: read the QR bytes off disk *before* enforcing
    /// the sandbox, install Landlock, run the in-memory decoder, write
    /// the tagged response, and `_exit` so the parent's destructors
    /// never run twice.
    fn child_main(path: &Path, write_fd: OwnedFd) -> ! {
        let qrcode_display = path.display().to_string();
        let result = (|| -> Result<String> {
            let bytes = std::fs::read(path)
                .with_context(|| i18n::error_cannot_read_file(&qrcode_display))?;
            apply_landlock()?;
            decode_qrcode_bytes(&bytes, &qrcode_display)
        })();

        let mut file = File::from(write_fd);
        let _ = match result {
            Ok(url) => write_tagged(&mut file, TAG_OK, url.as_bytes()),
            Err(err) => write_tagged(&mut file, TAG_ERR, format!("{err:#}").as_bytes()),
        };
        drop(file);

        // SAFETY: `_exit` immediately terminates the child without
        // running atexit handlers or destructors. That's what we want:
        // the parent owns the OS-visible state (config file, store
        // file, hook timestamps), and any drop in the forked child
        // would risk a double-flush.
        unsafe { libc::_exit(0) }
    }

    fn write_tagged(file: &mut File, tag: u8, body: &[u8]) -> std::io::Result<()> {
        file.write_all(&[tag])?;
        file.write_all(body)
    }

    /// Install a Landlock ruleset that denies every filesystem access
    /// right the kernel reports support for. The child has already read
    /// the QR bytes into memory at this point, so it does not need any
    /// path access; a successful enforce means an RCE in `image` or
    /// `rqrr` cannot open `~/.ssh/id_ed25519`, the encrypted store, or
    /// anything else.
    fn apply_landlock() -> Result<()> {
        let abi = ABI::V1;
        let access_all = AccessFs::from_all(abi);
        let status = Ruleset::default()
            .set_compatibility(CompatLevel::BestEffort)
            .handle_access(access_all)
            .context(i18n::error_qr_sandbox_failed())?
            .create()
            .context(i18n::error_qr_sandbox_failed())?
            .restrict_self()
            .context(i18n::error_qr_sandbox_failed())?;
        if status.ruleset == RulesetStatus::NotEnforced {
            log::warn!("{}", i18n::error_qr_sandbox_not_enforced());
        }
        Ok(())
    }

    // libc's WIFSIGNALED / WTERMSIG are C macros that don't have a
    // direct symbol exposed by the libc crate on every libc version, so
    // we replicate the (well-defined, POSIX) bit math here.

    const fn libc_wifsignaled(status: i32) -> bool {
        // Terminated by a signal: low 7 bits are the signal number,
        // and neither the "exited" (low byte == 0) nor "stopped"
        // (low byte == 0x7F) encodings match.
        let term_sig = status & 0x7F;
        term_sig != 0 && term_sig != 0x7F
    }

    const fn libc_wtermsig(status: i32) -> i32 {
        status & 0x7F
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn decode_response_returns_url_on_ok_tag() {
            let mut payload = vec![TAG_OK];
            payload.extend_from_slice(b"otpauth://totp/x?secret=AA");
            let url = decode_response(&payload, 0).unwrap();
            assert_eq!(url, "otpauth://totp/x?secret=AA");
        }

        #[test]
        fn decode_response_propagates_child_error_message() {
            let mut payload = vec![TAG_ERR];
            payload.extend_from_slice(b"boom");
            let err = decode_response(&payload, 0).unwrap_err();
            assert!(err.to_string().contains("boom"));
        }

        #[test]
        fn decode_response_fails_on_unknown_tag() {
            crate::cli::i18n::init_for_tests();
            let payload = [99u8, b'h', b'i'];
            assert!(decode_response(&payload, 0).is_err());
        }

        #[test]
        fn decode_response_fails_with_signal_status_on_empty_payload() {
            crate::cli::i18n::init_for_tests();
            // status = 11 means terminated by SIGSEGV (low 7 bits == 11,
            // not 0 and not 0x7F).
            let err = decode_response(&[], 11).unwrap_err();
            assert!(
                err.to_string().contains("11"),
                "signal number should appear in message, got: {err}",
            );
        }

        #[test]
        fn decode_response_fails_generically_on_empty_payload_with_clean_exit() {
            crate::cli::i18n::init_for_tests();
            // A "clean exit" status where the child wrote nothing: low
            // byte is zero, so WIFSIGNALED is false. We still want an
            // error because no URL was produced.
            assert!(decode_response(&[], 0).is_err());
        }

        #[test]
        fn decode_response_handles_invalid_utf8() {
            crate::cli::i18n::init_for_tests();
            let payload = [TAG_OK, 0xFFu8, 0xFEu8];
            assert!(decode_response(&payload, 0).is_err());
        }

        #[test]
        fn wifsignaled_detects_segv_status() {
            assert!(libc_wifsignaled(11));
            assert_eq!(libc_wtermsig(11), 11);
        }

        #[test]
        fn wifsignaled_is_false_for_normal_exit() {
            // Exit code 0: low byte is 0 → not signalled.
            assert!(!libc_wifsignaled(0));
            // Exit code 1 left-shifted (the way wait(2) encodes a
            // normal exit): low byte is 0 again → not signalled.
            assert!(!libc_wifsignaled(1 << 8));
        }

        #[test]
        fn wifsignaled_is_false_for_stopped_status() {
            // 0x7F in the low byte is the "stopped" marker, not a
            // signal termination.
            assert!(!libc_wifsignaled(0x7F));
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use super::*;

    /// PNG bytes of a known-good QR code encoding an otpauth URL. The
    /// fixture is the one used by the trycmd suite, so the unit tests
    /// and the snapshot tests share a single source of truth.
    fn known_good_qrcode_png() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/cmd/otp/add/add-totp-from-qrcode.in/qrcode.png");
        std::fs::read(path).expect("test QR PNG fixture must exist")
    }

    /// Encode a 4x4 white PNG — a valid image with no QR code inside.
    /// Used to drive the "no QR code found" code path with a buffer the
    /// `image` crate accepts.
    fn empty_png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 255, 255, 255]));
        let mut bytes: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("encoding a tiny PNG must succeed");
        bytes
    }

    #[test]
    fn decode_bytes_returns_otpauth_url_from_valid_png() {
        crate::cli::i18n::init_for_tests();
        let bytes = known_good_qrcode_png();
        let url = decode_qrcode_bytes(&bytes, "fixture.png").unwrap();
        assert!(
            url.starts_with("otpauth://"),
            "expected otpauth URL, got: {url}",
        );
    }

    #[test]
    fn decode_bytes_errors_on_non_image_input() {
        crate::cli::i18n::init_for_tests();
        let result = decode_qrcode_bytes(b"not an image at all", "junk.bin");
        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("Failed to decode QR code"),
            "expected decode error message, got: {message}",
        );
    }

    #[test]
    fn decode_bytes_errors_when_image_has_no_qrcode() {
        crate::cli::i18n::init_for_tests();
        let result = decode_qrcode_bytes(&empty_png_bytes(), "empty.png");
        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("No QR code found"),
            "expected no-QR message, got: {message}",
        );
    }

    #[test]
    fn from_file_errors_on_missing_path() {
        crate::cli::i18n::init_for_tests();
        let result = decode_qrcode_from_file(std::path::Path::new(
            "/definitely/does/not/exist/qrcode.png",
        ));
        assert!(result.is_err());
    }

    #[test]
    fn from_file_decodes_known_good_png() {
        crate::cli::i18n::init_for_tests();
        let tmp = TempDir::new().expect("create tempdir");
        let png = tmp.child("qrcode.png");
        png.write_binary(&known_good_qrcode_png())
            .expect("write png");
        let url = decode_qrcode_from_file(png.path()).unwrap();
        assert!(url.starts_with("otpauth://"));
    }

    /// End-to-end check of the public API. On Linux this exercises the
    /// fork+Landlock path; on other platforms it falls through to the
    /// in-process decoder. Either way the contract is identical.
    #[test]
    fn public_decoder_returns_otpauth_url_from_known_good_png() {
        crate::cli::i18n::init_for_tests();
        let tmp = TempDir::new().expect("create tempdir");
        let png = tmp.child("qrcode.png");
        png.write_binary(&known_good_qrcode_png())
            .expect("write png");
        let url = decode_qrcode_to_otpauth_url(png.path()).unwrap();
        assert!(url.starts_with("otpauth://"));
    }

    #[test]
    fn public_decoder_propagates_no_qrcode_error() {
        crate::cli::i18n::init_for_tests();
        let tmp = TempDir::new().expect("create tempdir");
        let png = tmp.child("empty.png");
        png.write_binary(&empty_png_bytes()).expect("write png");
        let err = decode_qrcode_to_otpauth_url(png.path()).unwrap_err();
        assert!(err.to_string().contains("No QR code found"));
    }

    #[test]
    fn public_decoder_errors_on_missing_file() {
        crate::cli::i18n::init_for_tests();
        let tmp = TempDir::new().expect("create tempdir");
        let missing = tmp.child("absent.png");
        let err = decode_qrcode_to_otpauth_url(missing.path()).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("Cannot read file") || message.contains("absent.png"),
            "expected file-read failure, got: {message}",
        );
    }
}
