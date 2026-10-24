//! Robust PDF text extraction.
//!
//! `pdf-extract` is built on `lopdf`, which does not handle every content
//! stream gracefully: `pdf-extract` `unwrap()`s the decoded stream and panics
//! when `lopdf` cannot parse it. The usual culprit is an inline image
//! (`BI ... ID ... EI`) that uses a filter such as `/FlateDecode`. `lopdf`
//! cannot decode filtered inline images, and its fallback — scanning for a
//! whitespace-delimited `EI` — can mis-locate the end of the image data. The
//! rest of the stream is then parsed as garbage and the whole stream fails to
//! decode.
//!
//! Inline images are decorative pixels; extracting text does not need them. We
//! therefore attempt a normal extraction (guarded against panics) and, if that
//! fails, rewrite every page content stream and Form XObject with the inline
//! images stripped out before retrying. The exact bounds of each inline image
//! are recovered by decompressing filtered data, so the surrounding text and
//! drawing operators survive untouched.

use anyhow::{Result, anyhow};
use flate2::{Decompress, FlushDecompress, Status};
use lopdf::{Document, Object, ObjectId};
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Extract the plain text from the PDF at `path`.
///
/// This never panics: a PDF that `pdf-extract` cannot handle yields an `Err`.
pub fn extract_text(path: &str) -> Result<String> {
    let direct_error = match guarded_extract(|| pdf_extract::extract_text(path)) {
        Ok(text) => return Ok(text),
        Err(error) => error,
    };

    // Retry against a copy of the document with the inline images removed so
    // that `lopdf` can decode the streams. `lopdf` can also panic while loading
    // or saving, so this is guarded too.
    let sanitised = match with_panics_silenced(|| sanitise(path)) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(error)) => {
            return Err(error.context("failed to rewrite the PDF to remove inline images"));
        }
        Err(_) => {
            return Err(anyhow!(
                "PDF text extraction failed: {direct_error}; the retry panicked"
            ));
        }
    };

    match guarded_extract(|| pdf_extract::extract_text_from_mem(&sanitised)) {
        Ok(text) => Ok(text),
        Err(retry_error) => Err(anyhow!(
            "PDF text extraction failed: {direct_error}; retrying without inline images also failed: {retry_error}"
        )),
    }
}

/// Run `extract`, converting a panic into an ordinary error.
///
/// `pdf-extract` panics rather than returning an error for streams it cannot
/// parse, so a panic must be caught to keep the CLI from aborting. The default
/// panic hook is temporarily silenced so the panic message does not leak to
/// stderr before we report a structured diagnostic.
fn guarded_extract<F>(extract: F) -> Result<String, String>
where
    F: FnOnce() -> Result<String, pdf_extract::OutputError>,
{
    match with_panics_silenced(extract) {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("`pdf-extract` panicked while decoding the document".to_string()),
    }
}

/// Run a closure with the panic hook temporarily silenced, catching panics.
fn with_panics_silenced<R>(f: impl FnOnce() -> R) -> std::thread::Result<R> {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(previous_hook);
    outcome
}

/// Load the PDF and return a copy with all inline images removed from content
/// streams and Form XObjects, serialised back to bytes.
fn sanitise(path: &str) -> Result<Vec<u8>> {
    let mut doc = Document::load(path)?;
    for id in content_stream_ids(&doc) {
        let Some(mut stream) = doc
            .get_object(id)
            .ok()
            .and_then(|object| object.as_stream().ok())
            .cloned()
        else {
            continue;
        };

        let content = if stream.filters().is_ok() {
            stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone())
        } else {
            stream.content.clone()
        };

        let (stripped, removed) = strip_inline_images(&content);
        if removed == 0 {
            continue;
        }

        stream.set_content(stripped);
        stream.dict.remove(b"Filter");
        stream.dict.remove(b"DecodeParms");
        doc.objects.insert(id, Object::Stream(stream));
    }

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes)?;
    Ok(bytes)
}

/// The object ids of every stream that `pdf-extract` will parse as a content
/// stream: page `/Contents` and every `/Subtype /Form` XObject.
fn content_stream_ids(doc: &Document) -> Vec<ObjectId> {
    let mut ids = HashSet::new();

    for (_, page_id) in doc.get_pages() {
        let Ok(page) = doc.get_object(page_id).and_then(|object| object.as_dict()) else {
            continue;
        };
        match page.get(b"Contents") {
            Ok(Object::Reference(id)) => {
                ids.insert(*id);
            }
            Ok(Object::Array(parts)) => {
                for part in parts {
                    if let Ok(id) = part.as_reference() {
                        ids.insert(id);
                    }
                }
            }
            _ => {}
        }
    }

    for (id, object) in &doc.objects {
        if let Ok(stream) = object.as_stream()
            && stream.dict.get(b"Subtype").and_then(|o| o.as_name()).ok()
                == Some(b"Form".as_slice())
        {
            ids.insert(*id);
        }
    }

    ids.into_iter().collect()
}

/// Remove every inline image (`BI ... ID <data> EI`) from a content stream,
/// returning the rewritten stream and the number of images removed.
///
/// Literal strings, hex strings and comments are skipped while scanning so that
/// a `BI` appearing in text is not mistaken for an image.
fn strip_inline_images(data: &[u8]) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(data.len());
    let mut copied = 0;
    let mut i = 0;
    let mut removed = 0;

    while i < data.len() {
        match data[i] {
            b'%' => {
                while i < data.len() && !matches!(data[i], b'\n' | b'\r') {
                    i += 1;
                }
            }
            b'(' => i = skip_literal_string(data, i),
            b'<' if data.get(i + 1) != Some(&b'<') => {
                i += 1;
                while i < data.len() && data[i] != b'>' {
                    i += 1;
                }
                i = (i + 1).min(data.len());
            }
            b'B' if is_inline_image_start(data, i) => match inline_image_end(data, i) {
                Some(end) => {
                    out.extend_from_slice(&data[copied..i]);
                    // Keep the streams token-separated where an image stood.
                    out.push(b' ');
                    copied = end;
                    i = end;
                    removed += 1;
                }
                // Cannot determine the bounds; leave the image for `lopdf`.
                None => i += 2,
            },
            _ => i += 1,
        }
    }

    out.extend_from_slice(&data[copied..]);
    (out, removed)
}

/// Return the index just past a literal string beginning at `start` (`(`).
fn skip_literal_string(data: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    let mut depth = 1u32;
    while i < data.len() {
        match data[i] {
            b'\\' => i += 2,
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    i.min(data.len())
}

/// Whether `data[i..]` begins the `BI` operator of an inline image.
fn is_inline_image_start(data: &[u8], i: usize) -> bool {
    if data.get(i..i + 2) != Some(b"BI") {
        return false;
    }
    let left_ok =
        i == 0 || !(data[i - 1].is_ascii_alphanumeric() || matches!(data[i - 1], b'*' | b'/'));
    let right_ok = matches!(
        data.get(i + 2),
        Some(byte) if byte.is_ascii_whitespace() || matches!(byte, b'/' | b'<')
    );
    left_ok && right_ok
}

/// Locate the end of the inline image beginning at `bi`, or `None` if the
/// bounds cannot be determined.
fn inline_image_end(data: &[u8], bi: usize) -> Option<usize> {
    let id = find_id_token(data, bi + 2)?;
    let dict = dict_tokens(&data[bi + 2..id]);
    let filtered = dict
        .iter()
        .any(|token| *token == b"F" || *token == b"Filter");

    let mut data_start = id + 2;
    if data.get(data_start).is_some_and(u8::is_ascii_whitespace) {
        data_start += 1;
    }

    let end = if filtered {
        data_start + zlib_consumed(&data[data_start..])?
    } else {
        let width = token_int(&dict, b"W")?;
        let height = token_int(&dict, b"H")?;
        let bits = token_int(&dict, b"BPC").unwrap_or(8);
        let components = token_name(&dict, b"CS")
            .and_then(colour_components)
            .unwrap_or(1);
        let stride = width
            .checked_mul(components)?
            .checked_mul(bits)?
            .div_ceil(8);
        data_start.checked_add(stride.checked_mul(height)?)?
    };

    // The image data is followed by optional whitespace and the `EI` operator.
    let mut cursor = end;
    while data.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if data.get(cursor..cursor + 2) == Some(b"EI") {
        return Some(cursor + 2);
    }

    // Fallback for image filters we cannot decode (e.g. ASCIIHexDecode).
    find_ei_marker(data, end)
}

/// Find a whitespace-delimited `EI` after `from`, returning the index past it.
fn find_ei_marker(data: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 3 <= data.len() {
        let delimited = data[i].is_ascii_whitespace()
            && &data[i + 1..i + 3] == b"EI"
            && (i + 3 == data.len() || data[i + 3].is_ascii_whitespace());
        if delimited {
            return Some(i + 3);
        }
        i += 1;
    }
    None
}

/// Find the whitespace-delimited `ID` token that ends the inline image dict.
fn find_id_token(data: &[u8], mut i: usize) -> Option<usize> {
    while i + 1 < data.len() {
        if &data[i..i + 2] == b"ID"
            && (i == 0 || data[i - 1].is_ascii_whitespace())
            && (i + 2 == data.len() || data[i + 2].is_ascii_whitespace())
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Split an inline image dictionary into tokens, treating PDF delimiters as
/// separators and stripping the leading slash from names.
fn dict_tokens(dict: &[u8]) -> Vec<&[u8]> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;

    for (i, &byte) in dict.iter().enumerate() {
        let delimiter = byte.is_ascii_whitespace()
            || matches!(byte, b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')');
        if delimiter {
            if let Some(start) = start.take() {
                tokens.push(&dict[start..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(start) = start {
        tokens.push(&dict[start..]);
    }
    tokens
}

/// The integer value following the `/key` entry, if present.
fn token_int(tokens: &[&[u8]], key: &[u8]) -> Option<usize> {
    let index = tokens.iter().position(|token| *token == key)?;
    std::str::from_utf8(tokens.get(index + 1)?)
        .ok()?
        .parse()
        .ok()
}

/// The name value following the `/key` entry, if present.
fn token_name<'a>(tokens: &[&'a [u8]], key: &[u8]) -> Option<&'a [u8]> {
    let index = tokens.iter().position(|token| *token == key)?;
    tokens.get(index + 1).copied()
}

/// The number of colour components for a (possibly abbreviated) colour space.
fn colour_components(name: &[u8]) -> Option<usize> {
    match name {
        b"DeviceGray" | b"Gray" | b"G" => Some(1),
        b"DeviceRGB" | b"RGB" => Some(3),
        b"DeviceCMYK" | b"CMYK" => Some(4),
        b"DeviceRGBA" | b"RGBA" => Some(4),
        _ => None,
    }
}

/// Decompress a zlib stream and report how many input bytes it consumed.
fn zlib_consumed(data: &[u8]) -> Option<usize> {
    let mut decompressor = Decompress::new(true);
    let mut scratch = vec![0u8; 64 * 1024];

    loop {
        let input_before = decompressor.total_in() as usize;
        let output_before = decompressor.total_out() as usize;
        let status = decompressor
            .decompress(&data[input_before..], &mut scratch, FlushDecompress::None)
            .ok()?;

        if status == Status::StreamEnd {
            return Some(decompressor.total_in() as usize);
        }
        // No forward progress means the stream is truncated or malformed.
        if decompressor.total_in() as usize == input_before
            && decompressor.total_out() as usize == output_before
        {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    fn deflate(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn strips_unfiltered_inline_image() {
        let stream =
            b"q BI /CS /RGB /W 2 /H 1 /BPC 8 ID \x01\x02\x03\x04\x05\x06 EI Q BT (hi) Tj ET";
        let (stripped, removed) = strip_inline_images(stream);
        assert_eq!(removed, 1);
        let text = String::from_utf8_lossy(&stripped);
        assert!(!text.contains("BI"), "image marker survived: {text:?}");
        assert!(text.contains("(hi) Tj"), "text was lost: {text:?}");
    }

    #[test]
    fn strips_filtered_inline_image() {
        let compressed = deflate(&[0u8; 30]);
        let mut stream = b"q\nBI\n/CS/RGB\n/W 10\n/H 1\n/BPC 8\n/F/Fl\nID ".to_vec();
        stream.extend_from_slice(&compressed);
        stream.extend_from_slice(b"\nEI Q\nBT (hello) Tj ET");

        let (stripped, removed) = strip_inline_images(&stream);
        assert_eq!(removed, 1);
        let text = String::from_utf8_lossy(&stripped);
        assert!(!text.contains("/CS"), "image dict survived: {text:?}");
        assert!(text.contains("(hello) Tj"), "text was lost: {text:?}");
    }

    #[test]
    fn ignores_bi_inside_literal_string() {
        let stream = b"BT (BI /CS /RGB) Tj ET";
        let (stripped, removed) = strip_inline_images(stream);
        assert_eq!(removed, 0);
        assert_eq!(stripped, stream);
    }

    #[test]
    fn leaves_plain_content_untouched() {
        let stream = b"BT /F1 12 Tf 72 720 Td (Hello, world!) Tj ET";
        let (stripped, removed) = strip_inline_images(stream);
        assert_eq!(removed, 0);
        assert_eq!(stripped, stream);
    }
}
