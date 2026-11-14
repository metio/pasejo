// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Streaming replacement of secret values in a child's output.
//!
//! Output arrives in arbitrary chunks, so a value can straddle two reads.
//! A [`RedactingStream`] holds back the tail of what it has seen only while
//! that tail could still grow into a value, and passes everything else
//! through at once. Output that merely starts like a value waits for the
//! next chunk, or for the end of the stream, before it is written.

use std::io::{self, Read, Write};

use zeroize::Zeroizing;

/// Values shorter than this are not redacted: replacing every occurrence of
/// one or two characters would mangle the output beyond use.
pub const MIN_REDACTED_LEN: usize = 3;

struct Needle {
    value: Zeroizing<Vec<u8>>,
    replacement: Vec<u8>,
}

/// The values to redact, each with the text that replaces it.
pub struct Redactor {
    /// Sorted longest first, so a value that contains another one is
    /// replaced as a whole.
    needles: Vec<Needle>,
}

impl Redactor {
    /// `values` pairs each replacement with the value it stands for. Values
    /// shorter than [`MIN_REDACTED_LEN`] bytes are left out.
    pub fn new<'a>(values: impl IntoIterator<Item = (&'a str, String)>) -> Self {
        let mut needles: Vec<Needle> = values
            .into_iter()
            .filter(|(value, _)| value.len() >= MIN_REDACTED_LEN)
            .map(|(value, replacement)| Needle {
                value: Zeroizing::new(value.as_bytes().to_vec()),
                replacement: replacement.into_bytes(),
            })
            .collect();
        needles.sort_by_key(|needle| std::cmp::Reverse(needle.value.len()));
        Self { needles }
    }

    pub fn stream(&self) -> RedactingStream<'_> {
        RedactingStream {
            redactor: self,
            pending: Zeroizing::new(Vec::new()),
        }
    }

    /// Copies `input` to `output` until `input` ends, redacting on the way.
    /// `output` is flushed after every chunk so interactive output is not
    /// delayed beyond what redaction needs.
    pub fn copy(&self, mut input: impl Read, mut output: impl Write) -> io::Result<()> {
        let mut stream = self.stream();
        let mut chunk = Zeroizing::new(vec![0_u8; 8192]);
        let mut redacted = Vec::new();
        loop {
            let read = match input.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            redacted.clear();
            stream.push(chunk.get(..read).unwrap_or_default(), &mut redacted);
            output.write_all(&redacted)?;
            output.flush()?;
        }
        redacted.clear();
        stream.finish(&mut redacted);
        output.write_all(&redacted)?;
        output.flush()
    }
}

/// One output stream passing through a [`Redactor`].
pub struct RedactingStream<'a> {
    redactor: &'a Redactor,
    /// Bytes read but not yet written, because they could be the start of
    /// a value. They may hold part of a secret, hence the zeroizing buffer.
    pending: Zeroizing<Vec<u8>>,
}

impl RedactingStream<'_> {
    /// Appends the redacted form of everything in `input` that can no
    /// longer turn into a value to `output`.
    pub fn push(&mut self, input: &[u8], output: &mut Vec<u8>) {
        self.pending.extend_from_slice(input);
        self.drain(output, false);
    }

    /// Appends the rest to `output` at the end of the stream, where a
    /// partial value can no longer complete.
    pub fn finish(&mut self, output: &mut Vec<u8>) {
        self.drain(output, true);
    }

    fn drain(&mut self, output: &mut Vec<u8>, at_end: bool) {
        let needles = &self.redactor.needles;
        let mut position = 0;
        while let Some(rest) = self.pending.get(position..).filter(|rest| !rest.is_empty()) {
            let could_grow =
                |needle: &Needle| needle.value.len() > rest.len() && needle.value.starts_with(rest);
            if !at_end && needles.iter().any(could_grow) {
                break;
            }
            if let Some(needle) = needles
                .iter()
                .find(|needle| rest.starts_with(&needle.value))
            {
                output.extend_from_slice(&needle.replacement);
                position += needle.value.len();
            } else {
                output.extend_from_slice(rest.get(..1).unwrap_or_default());
                position += 1;
            }
        }
        self.pending.drain(..position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redactor(values: &[(&'static str, &str)]) -> Redactor {
        Redactor::new(
            values
                .iter()
                .map(|(value, replacement)| (*value, (*replacement).to_owned())),
        )
    }

    fn redact_chunks(redactor: &Redactor, chunks: &[&str]) -> String {
        let mut stream = redactor.stream();
        let mut output = Vec::new();
        for chunk in chunks {
            stream.push(chunk.as_bytes(), &mut output);
        }
        stream.finish(&mut output);
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn replaces_every_occurrence_of_a_value() {
        let redactor = redactor(&[("hunter2", "${PASSWORD}")]);
        assert_eq!(
            redact_chunks(&redactor, &["a hunter2 b hunter2"]),
            "a ${PASSWORD} b ${PASSWORD}"
        );
    }

    #[test]
    fn replaces_a_value_split_across_chunks() {
        let redactor = redactor(&[("hunter2", "${PASSWORD}")]);
        assert_eq!(
            redact_chunks(&redactor, &["login hun", "te", "r2 done"]),
            "login ${PASSWORD} done"
        );
    }

    #[test]
    fn writes_text_that_cannot_become_a_value_immediately() {
        let redactor = redactor(&[("hunter2", "${PASSWORD}")]);
        let mut stream = redactor.stream();
        let mut output = Vec::new();
        stream.push(b"Password: ", &mut output);
        assert_eq!(output, b"Password: ");
    }

    #[test]
    fn holds_back_only_a_possible_start_of_a_value() {
        let redactor = redactor(&[("hunter2", "${PASSWORD}")]);
        let mut stream = redactor.stream();
        let mut output = Vec::new();
        stream.push(b"say hun", &mut output);
        assert_eq!(output, b"say ");
        stream.push(b"gry", &mut output);
        assert_eq!(output, b"say hungry");
    }

    #[test]
    fn writes_a_partial_value_at_the_end_of_the_stream() {
        let redactor = redactor(&[("hunter2", "${PASSWORD}")]);
        assert_eq!(
            redact_chunks(&redactor, &["ends with hunt"]),
            "ends with hunt"
        );
    }

    #[test]
    fn replaces_the_longest_value_that_matches() {
        let redactor = redactor(&[("abc", "${SHORT}"), ("abcdef", "${LONG}")]);
        assert_eq!(
            redact_chunks(&redactor, &["abcdef abc"]),
            "${LONG} ${SHORT}"
        );
        assert_eq!(redact_chunks(&redactor, &["ab", "cd", "ef"]), "${LONG}");
        assert_eq!(redact_chunks(&redactor, &["abcde", "x"]), "${SHORT}dex");
    }

    #[test]
    fn leaves_values_shorter_than_the_minimum_alone() {
        let redactor = redactor(&[("ab", "${SHORT}")]);
        assert_eq!(redact_chunks(&redactor, &["ab ab"]), "ab ab");
    }

    #[test]
    fn handles_multibyte_values_split_inside_a_character() {
        let redactor = redactor(&[("pässwört", "${PASSWORD}")]);
        let mut stream = redactor.stream();
        let mut output = Vec::new();
        for byte in "x pässwört y".bytes() {
            stream.push(&[byte], &mut output);
        }
        stream.finish(&mut output);
        assert_eq!(String::from_utf8(output).unwrap(), "x ${PASSWORD} y");
    }

    #[test]
    fn copy_redacts_a_whole_reader() {
        let redactor = redactor(&[("hunter2", "${PASSWORD}")]);
        let mut output = Vec::new();
        redactor
            .copy("one hunter2\ntwo hunter2\n".as_bytes(), &mut output)
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "one ${PASSWORD}\ntwo ${PASSWORD}\n"
        );
    }
}
