//! Pure-Rust Zero-Dependency PNG & RFC 1950/1951 Zlib/Deflate Decoder for Embedded glTF 2.0
//! (`.glb`) Textures (`crates/satya-web/src/wasm_client/glb_png`).
//!
//! Decodes 8-bit non-interlaced PNG images (color types `0` Gray, `2` RGB, `3` Indexed/`PLTE`
//! with optional `tRNS`, `4` Gray+Alpha, and `6` RGBA) into tight top-down `Rgba8` pixel buffers
//! ready for WebGPU `wgpu::TextureFormat::Rgba8Unorm` upload.

const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const MAX_PNG_DIM: u32 = 4096;
const MAX_INFLATED_BYTES: usize = 64 * 1024 * 1024;

/// Decoded top-down RGBA8 image buffer extracted from an embedded `.glb` PNG image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPng {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

/// Decodes an 8-bit non-interlaced PNG byte slice into a tight top-down `Rgba8` buffer.
pub fn decode_png_rgba8(bytes: &[u8]) -> Result<DecodedPng, String> {
    if bytes.len() < 8 || bytes[..8] != PNG_SIG {
        return Err("Invalid PNG signature".to_string());
    }

    let mut offset = 8_usize;
    let mut ihdr: Option<(u32, u32, u8, u8)> = None;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    let mut idat: Vec<u8> = Vec::new();
    let mut saw_iend = false;

    while offset + 12 <= bytes.len() {
        let len = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let ctype = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start
            .checked_add(len)
            .ok_or_else(|| "PNG chunk length overflow".to_string())?;
        let crc_end = data_end
            .checked_add(4)
            .ok_or_else(|| "PNG chunk CRC overflow".to_string())?;
        if crc_end > bytes.len() {
            return Err("Truncated PNG chunk".to_string());
        }
        let chunk = &bytes[data_start..data_end];

        match ctype {
            b"IHDR" => {
                if chunk.len() != 13 {
                    return Err("Invalid IHDR chunk length".to_string());
                }
                let w = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let h = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                let bit_depth = chunk[8];
                let color_type = chunk[9];
                let compression = chunk[10];
                let filter_method = chunk[11];
                let interlace = chunk[12];
                if w == 0 || h == 0 || w > MAX_PNG_DIM || h > MAX_PNG_DIM {
                    return Err(format!("Unsupported PNG dimensions {w}x{h}"));
                }
                if bit_depth != 8 {
                    return Err(format!(
                        "Unsupported PNG bit depth {bit_depth} (expected 8-bit)"
                    ));
                }
                if compression != 0 || filter_method != 0 {
                    return Err("Unsupported PNG compression or filter method".to_string());
                }
                if interlace != 0 {
                    return Err("Interlaced PNG images are not supported".to_string());
                }
                ihdr = Some((w, h, bit_depth, color_type));
            }
            b"PLTE" => {
                if !chunk.len().is_multiple_of(3) {
                    return Err("Invalid PLTE chunk length".to_string());
                }
                palette = chunk.as_chunks::<3>().0.to_vec();
            }
            b"tRNS" => {
                trns = chunk.to_vec();
            }
            b"IDAT" => {
                idat.extend_from_slice(chunk);
            }
            b"IEND" => {
                saw_iend = true;
                break;
            }
            _ => {}
        }
        offset = crc_end;
    }

    let (width, height, _depth, color_type) =
        ihdr.ok_or_else(|| "Missing IHDR chunk in PNG".to_string())?;
    if !saw_iend && idat.is_empty() {
        return Err("Missing IDAT/IEND chunks in PNG".to_string());
    }

    let bpp = match color_type {
        0 | 3 => 1_usize,
        2 => 3_usize,
        4 => 2_usize,
        6 => 4_usize,
        other => return Err(format!("Unsupported PNG color type {other}")),
    };
    if color_type == 3 && palette.is_empty() {
        return Err("Indexed PNG missing PLTE chunk".to_string());
    }

    let row_bytes = (width as usize)
        .checked_mul(bpp)
        .ok_or_else(|| "PNG row stride overflow".to_string())?;
    let expected_raw = (height as usize)
        .checked_mul(1 + row_bytes)
        .ok_or_else(|| "PNG buffer size overflow".to_string())?;
    if expected_raw > MAX_INFLATED_BYTES {
        return Err("PNG image exceeds 64 MiB uncompressed safety limit".to_string());
    }

    let raw = decompress_zlib(&idat, expected_raw)?;
    if raw.len() < expected_raw {
        return Err(format!(
            "Truncated PNG IDAT stream: got {} bytes, expected {expected_raw}",
            raw.len()
        ));
    }

    let mut prev_row = vec![0_u8; row_bytes];
    let mut cur_row = vec![0_u8; row_bytes];
    let mut rgba8 = vec![0_u8; (width as usize) * (height as usize) * 4];

    for y in 0..(height as usize) {
        let src_start = y * (1 + row_bytes);
        let filter_type = raw[src_start];
        let src = &raw[src_start + 1..src_start + 1 + row_bytes];
        unfilter_scanline(filter_type, src, &prev_row, &mut cur_row, bpp)?;
        expand_row_to_rgba8(
            color_type,
            &cur_row,
            &palette,
            &trns,
            &mut rgba8[y * (width as usize) * 4..(y + 1) * (width as usize) * 4],
        )?;
        prev_row.copy_from_slice(&cur_row);
    }

    Ok(DecodedPng {
        width,
        height,
        rgba8,
    })
}

fn unfilter_scanline(
    filter: u8,
    src: &[u8],
    prev: &[u8],
    dst: &mut [u8],
    bpp: usize,
) -> Result<(), String> {
    match filter {
        0 => dst.copy_from_slice(src),
        1 => {
            for i in 0..src.len() {
                let left = if i >= bpp { dst[i - bpp] } else { 0 };
                dst[i] = src[i].wrapping_add(left);
            }
        }
        2 => {
            for i in 0..src.len() {
                dst[i] = src[i].wrapping_add(prev[i]);
            }
        }
        3 => {
            for i in 0..src.len() {
                let left = if i >= bpp { u16::from(dst[i - bpp]) } else { 0 };
                let up = u16::from(prev[i]);
                dst[i] = src[i].wrapping_add(((left + up) >> 1) as u8);
            }
        }
        4 => {
            for i in 0..src.len() {
                let a = if i >= bpp { dst[i - bpp] } else { 0 };
                let b = prev[i];
                let c = if i >= bpp { prev[i - bpp] } else { 0 };
                dst[i] = src[i].wrapping_add(paeth_predictor(a, b, c));
            }
        }
        other => return Err(format!("Unsupported PNG scanline filter {other}")),
    }
    Ok(())
}

#[inline]
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let ai = i32::from(a);
    let bi = i32::from(b);
    let ci = i32::from(c);
    let p = ai + bi - ci;
    let pa = (p - ai).abs();
    let pb = (p - bi).abs();
    let pc = (p - ci).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn expand_row_to_rgba8(
    color_type: u8,
    row: &[u8],
    palette: &[[u8; 3]],
    trns: &[u8],
    out: &mut [u8],
) -> Result<(), String> {
    match color_type {
        0 => {
            let trans_gray = if trns.len() >= 2 {
                Some(trns[1])
            } else {
                None
            };
            for (i, &g) in row.iter().enumerate() {
                let a = if trans_gray == Some(g) { 0 } else { 255 };
                out[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, a]);
            }
        }
        2 => {
            let trans_rgb = if trns.len() >= 6 {
                Some([trns[1], trns[3], trns[5]])
            } else {
                None
            };
            for (i, px) in row.as_chunks::<3>().0.iter().enumerate() {
                let rgb = [px[0], px[1], px[2]];
                let a = if trans_rgb == Some(rgb) { 0 } else { 255 };
                out[i * 4..i * 4 + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
            }
        }
        3 => {
            for (i, &idx) in row.iter().enumerate() {
                let rgb = palette
                    .get(idx as usize)
                    .copied()
                    .ok_or_else(|| format!("PLTE index {idx} out of bounds"))?;
                let a = trns.get(idx as usize).copied().unwrap_or(255);
                out[i * 4..i * 4 + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
            }
        }
        4 => {
            for (i, px) in row.as_chunks::<2>().0.iter().enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        6 => {
            out.copy_from_slice(row);
        }
        other => return Err(format!("Unsupported PNG color type {other}")),
    }
    Ok(())
}

// ============================================================================
// RFC 1950 Zlib + RFC 1951 DEFLATE Inflater
// ============================================================================

struct BitReader<'a> {
    bytes: &'a [u8],
    byte_pos: usize,
    bit_buf: u32,
    bit_cnt: u8,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_pos: 0,
            bit_buf: 0,
            bit_cnt: 0,
        }
    }

    fn ensure_bits(&mut self, n: u8) -> Result<(), String> {
        while self.bit_cnt < n {
            let b = *self
                .bytes
                .get(self.byte_pos)
                .ok_or_else(|| "Unexpected end of DEFLATE stream".to_string())?;
            self.byte_pos += 1;
            self.bit_buf |= u32::from(b) << self.bit_cnt;
            self.bit_cnt += 8;
        }
        Ok(())
    }

    fn read_bits(&mut self, n: u8) -> Result<u32, String> {
        if n == 0 {
            return Ok(0);
        }
        self.ensure_bits(n)?;
        let val = self.bit_buf & ((1_u32 << n) - 1);
        self.bit_buf >>= n;
        self.bit_cnt -= n;
        Ok(val)
    }

    fn align_to_byte(&mut self) {
        let drop = self.bit_cnt & 7;
        self.bit_buf >>= drop;
        self.bit_cnt -= drop;
    }

    fn read_aligned_bytes(&mut self, count: usize) -> Result<&'a [u8], String> {
        self.align_to_byte();
        let buffered_bytes = usize::from(self.bit_cnt / 8);
        self.byte_pos = self.byte_pos.saturating_sub(buffered_bytes);
        self.bit_buf = 0;
        self.bit_cnt = 0;
        let end = self
            .byte_pos
            .checked_add(count)
            .ok_or_else(|| "DEFLATE byte slice overflow".to_string())?;
        let slice = self
            .bytes
            .get(self.byte_pos..end)
            .ok_or_else(|| "Truncated uncompressed DEFLATE block".to_string())?;
        self.byte_pos = end;
        Ok(slice)
    }
}

struct HuffmanTree {
    max_bits: usize,
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl HuffmanTree {
    fn from_code_lengths(lengths: &[u8]) -> Result<Self, String> {
        let mut counts = [0_u16; 16];
        let mut max_bits = 0_usize;
        for &len in lengths {
            let l = len as usize;
            if l > 15 {
                return Err("Huffman code length > 15".to_string());
            }
            if l > 0 {
                counts[l] += 1;
                max_bits = max_bits.max(l);
            }
        }
        if max_bits == 0 {
            return Err("Empty Huffman tree".to_string());
        }

        let mut offsets = [0_u16; 16];
        let mut total = 0_u16;
        for bits in 1..=15 {
            offsets[bits] = total;
            total = total
                .checked_add(counts[bits])
                .ok_or_else(|| "Huffman symbol count overflow".to_string())?;
        }

        let mut left = 1_i32;
        for &count in counts.iter().take(max_bits + 1).skip(1) {
            left = (left << 1) - i32::from(count);
            if left < 0 {
                return Err("Over-subscribed Huffman tree".to_string());
            }
        }

        let mut symbols = vec![0_u16; total as usize];
        for (sym, &len) in lengths.iter().enumerate() {
            let l = len as usize;
            if l > 0 {
                let idx = offsets[l] as usize;
                offsets[l] += 1;
                symbols[idx] = sym as u16;
            }
        }

        Ok(Self {
            max_bits,
            counts,
            symbols,
        })
    }

    fn decode_symbol(&self, reader: &mut BitReader<'_>) -> Result<u16, String> {
        let mut code = 0_i32;
        let mut first = 0_i32;
        let mut index = 0_i32;
        for len in 1..=self.max_bits {
            code |= reader.read_bits(1)? as i32;
            let count = i32::from(self.counts[len]);
            if code - count < first {
                let sym_idx = (index + (code - first)) as usize;
                return self
                    .symbols
                    .get(sym_idx)
                    .copied()
                    .ok_or_else(|| "Huffman decode index out of bounds".to_string());
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err("Invalid Huffman code in DEFLATE stream".to_string())
    }
}

fn fixed_trees() -> Result<(HuffmanTree, HuffmanTree), String> {
    let mut lit_lengths = [0_u8; 288];
    lit_lengths[0..144].fill(8);
    lit_lengths[144..256].fill(9);
    lit_lengths[256..280].fill(7);
    lit_lengths[280..288].fill(8);
    let dist_lengths = [5_u8; 32];
    Ok((
        HuffmanTree::from_code_lengths(&lit_lengths)?,
        HuffmanTree::from_code_lengths(&dist_lengths)?,
    ))
}

fn dynamic_trees(reader: &mut BitReader<'_>) -> Result<(HuffmanTree, HuffmanTree), String> {
    const CL_ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let hlit = (reader.read_bits(5)? as usize) + 257;
    let hdist = (reader.read_bits(5)? as usize) + 1;
    let hclen = (reader.read_bits(4)? as usize) + 4;
    if hlit > 286 || hdist > 30 {
        return Err("Invalid dynamic DEFLATE header lengths".to_string());
    }

    let mut cl_lengths = [0_u8; 19];
    for i in 0..hclen {
        cl_lengths[CL_ORDER[i]] = reader.read_bits(3)? as u8;
    }
    let cl_tree = HuffmanTree::from_code_lengths(&cl_lengths)?;

    let total = hlit + hdist;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        let sym = cl_tree.decode_symbol(reader)?;
        match sym {
            0..=15 => lengths.push(sym as u8),
            16 => {
                let prev = *lengths
                    .last()
                    .ok_or_else(|| "DEFLATE RLE code 16 with no previous length".to_string())?;
                let rep = (reader.read_bits(2)? as usize) + 3;
                if lengths.len() + rep > total {
                    return Err("DEFLATE code length repeat overflow".to_string());
                }
                lengths.resize(lengths.len() + rep, prev);
            }
            17 => {
                let rep = (reader.read_bits(3)? as usize) + 3;
                if lengths.len() + rep > total {
                    return Err("DEFLATE zero repeat 17 overflow".to_string());
                }
                lengths.resize(lengths.len() + rep, 0);
            }
            18 => {
                let rep = (reader.read_bits(7)? as usize) + 11;
                if lengths.len() + rep > total {
                    return Err("DEFLATE zero repeat 18 overflow".to_string());
                }
                lengths.resize(lengths.len() + rep, 0);
            }
            _ => return Err("Invalid code length symbol in DEFLATE".to_string()),
        }
    }

    let lit_tree = HuffmanTree::from_code_lengths(&lengths[..hlit])?;
    let dist_tree = HuffmanTree::from_code_lengths(&lengths[hlit..])?;
    Ok((lit_tree, dist_tree))
}

fn decompress_zlib(zlib: &[u8], capacity_hint: usize) -> Result<Vec<u8>, String> {
    if zlib.len() < 6 {
        return Err("Truncated zlib stream".to_string());
    }
    let cmf = zlib[0];
    let flg = zlib[1];
    if (cmf & 0x0F) != 8 {
        return Err(format!("Unsupported zlib compression method {}", cmf & 0x0F));
    }
    if ((u16::from(cmf) << 8) | u16::from(flg)) % 31 != 0 {
        return Err("Invalid zlib header check bits".to_string());
    }
    let start = if (flg & 0x20) != 0 { 6 } else { 2 };
    if start >= zlib.len() {
        return Err("Missing zlib DEFLATE payload".to_string());
    }

    const LEN_BASE: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LEN_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DIST_BASE: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    let mut reader = BitReader::new(&zlib[start..]);
    let mut out = Vec::with_capacity(capacity_hint.min(MAX_INFLATED_BYTES));

    loop {
        let bfinal = reader.read_bits(1)?;
        let btype = reader.read_bits(2)?;
        match btype {
            0 => {
                let hdr = reader.read_aligned_bytes(4)?;
                let len = u16::from_le_bytes([hdr[0], hdr[1]]);
                let nlen = u16::from_le_bytes([hdr[2], hdr[3]]);
                if len ^ 0xFFFF != nlen {
                    return Err("Corrupt stored DEFLATE block length".to_string());
                }
                let data = reader.read_aligned_bytes(usize::from(len))?;
                if out.len() + data.len() > MAX_INFLATED_BYTES {
                    return Err("DEFLATE output exceeded 64 MiB limit".to_string());
                }
                out.extend_from_slice(data);
            }
            1 | 2 => {
                let (lit_tree, dist_tree) = if btype == 1 {
                    fixed_trees()?
                } else {
                    dynamic_trees(&mut reader)?
                };
                loop {
                    let sym = lit_tree.decode_symbol(&mut reader)?;
                    if sym < 256 {
                        if out.len() >= MAX_INFLATED_BYTES {
                            return Err("DEFLATE output exceeded 64 MiB limit".to_string());
                        }
                        out.push(sym as u8);
                    } else if sym == 256 {
                        break;
                    } else {
                        let len_idx = usize::from(sym - 257);
                        let &base_len = LEN_BASE
                            .get(len_idx)
                            .ok_or_else(|| "Invalid DEFLATE length code".to_string())?;
                        let extra_l = LEN_EXTRA[len_idx];
                        let match_len = usize::from(base_len) + (reader.read_bits(extra_l)? as usize);

                        let dist_sym = dist_tree.decode_symbol(&mut reader)? as usize;
                        let &base_dist = DIST_BASE
                            .get(dist_sym)
                            .ok_or_else(|| "Invalid DEFLATE distance code".to_string())?;
                        let extra_d = DIST_EXTRA[dist_sym];
                        let dist = usize::from(base_dist) + (reader.read_bits(extra_d)? as usize);

                        if dist == 0 || dist > out.len() {
                            return Err("Invalid DEFLATE back-reference distance".to_string());
                        }
                        if out.len() + match_len > MAX_INFLATED_BYTES {
                            return Err("DEFLATE output exceeded 64 MiB limit".to_string());
                        }
                        for _ in 0..match_len {
                            let b = out[out.len() - dist];
                            out.push(b);
                        }
                    }
                }
            }
            other => return Err(format!("Invalid DEFLATE block type {other}")),
        }
        if bfinal == 1 {
            break;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adler32(data: &[u8]) -> u32 {
        let mut s1 = 1_u32;
        let mut s2 = 0_u32;
        for &b in data {
            s1 = (s1 + u32::from(b)) % 65521;
            s2 = (s2 + s1) % 65521;
        }
        (s2 << 16) | s1
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFF_u32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(tag);
        out.extend_from_slice(payload);
        let mut crc_buf = Vec::with_capacity(4 + payload.len());
        crc_buf.extend_from_slice(tag);
        crc_buf.extend_from_slice(payload);
        out.extend_from_slice(&crc32(&crc_buf).to_be_bytes());
    }

    fn build_uncompressed_png(width: u32, height: u32, color_type: u8, raw_scanlines: &[u8]) -> Vec<u8> {
        let mut zlib = vec![0x78, 0x01];
        let mut pos = 0_usize;
        while pos < raw_scanlines.len() {
            let chunk_len = (raw_scanlines.len() - pos).min(65535);
            let is_last = pos + chunk_len >= raw_scanlines.len();
            zlib.push(u8::from(is_last));
            let len_u16 = chunk_len as u16;
            zlib.extend_from_slice(&len_u16.to_le_bytes());
            zlib.extend_from_slice(&(!len_u16).to_le_bytes());
            zlib.extend_from_slice(&raw_scanlines[pos..pos + chunk_len]);
            pos += chunk_len;
        }
        zlib.extend_from_slice(&adler32(raw_scanlines).to_be_bytes());

        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);

        let mut png = Vec::from(PNG_SIG);
        push_chunk(&mut png, b"IHDR", &ihdr);
        push_chunk(&mut png, b"IDAT", &zlib);
        push_chunk(&mut png, b"IEND", &[]);
        png
    }

    #[test]
    fn test_decode_png_rgba8_filters_and_rgb_expansion() {
        // 2x2 RGBA image using filter 1 (Sub) on row 0 and filter 4 (Paeth) on row 1
        let expected_rgba = vec![
            10, 20, 30, 255, 40, 60, 80, 200, // row 0
            15, 25, 35, 255, 45, 65, 85, 200, // row 1
        ];
        let mut raw = Vec::new();
        // Row 0 (Sub): first pixel raw, second pixel diff from left
        raw.push(1);
        raw.extend_from_slice(&[10, 20, 30, 255, 30, 40, 50, 201]);
        // Row 1 (Paeth): pixel 0 predicts b=10 -> +5 = 15; pixel 1 predicts b=40 -> +5 = 45
        raw.push(4);
        raw.extend_from_slice(&[5, 5, 5, 0, 5, 5, 5, 0]);

        let png_rgba = build_uncompressed_png(2, 2, 6, &raw);
        let dec = decode_png_rgba8(&png_rgba).expect("RGBA PNG should decode");
        assert_eq!(dec.width, 2);
        assert_eq!(dec.height, 2);
        assert_eq!(dec.rgba8, expected_rgba);

        // 2x2 RGB image using filter 0 (None), filter 2 (Up), and filter 3 (Average)
        let mut raw_rgb = Vec::new();
        raw_rgb.push(0);
        raw_rgb.extend_from_slice(&[100, 120, 140, 160, 180, 200]);
        raw_rgb.push(2);
        raw_rgb.extend_from_slice(&[10, 10, 10, 20, 20, 20]);
        let png_rgb = build_uncompressed_png(2, 2, 2, &raw_rgb);
        let dec_rgb = decode_png_rgba8(&png_rgb).expect("RGB PNG should decode");
        assert_eq!(
            dec_rgb.rgba8,
            vec![
                100, 120, 140, 255, 160, 180, 200, 255, // row 0
                110, 130, 150, 255, 180, 200, 220, 255, // row 1
            ]
        );

        let mut raw_avg = Vec::new();
        raw_avg.push(0);
        raw_avg.extend_from_slice(&[100, 120, 140, 160, 180, 200]);
        raw_avg.push(3);
        // pixel 0 avg((0, 100)) = 50 -> +10 = 60; pixel 1 avg((60, 160)) = 110 -> +20 = 130
        raw_avg.extend_from_slice(&[10, 10, 10, 20, 20, 20]);
        let png_avg = build_uncompressed_png(2, 2, 2, &raw_avg);
        let dec_avg = decode_png_rgba8(&png_avg).expect("Average-filtered RGB PNG should decode");
        assert_eq!(
            dec_avg.rgba8,
            vec![
                100, 120, 140, 255, 160, 180, 200, 255, // row 0
                60, 70, 80, 255, 130, 145, 160, 255, // row 1
            ]
        );
    }

    #[test]
    fn test_decode_png_fixed_huffman_and_error_guards() {
        // Build a minimal fixed-Huffman zlib payload encoding:
        // filter 0, RGB(12, 34, 56), RGB(12, 34, 56) via match len=3, dist=3
        // Fixed Huffman codes (LSB-first in stream, bits of code MSB-first):
        // lit 0..=143 -> 8 bits: 00110000 + val
        // lit 256 -> 7 bits: 0000000
        // lit 257 (len=3) -> 7 bits: 0000001
        // dist 2 (base=3) -> 5 bits: 00010
        fn push_bits_msb(bits: &mut Vec<u8>, val: u32, n: u8) {
            for i in (0..n).rev() {
                bits.push(((val >> i) & 1) as u8);
            }
        }
        fn push_bits_lsb(bits: &mut Vec<u8>, val: u32, n: u8) {
            for i in 0..n {
                bits.push(((val >> i) & 1) as u8);
            }
        }
        let mut bit_stream = Vec::new();
        push_bits_lsb(&mut bit_stream, 1, 1); // bfinal = 1
        push_bits_lsb(&mut bit_stream, 1, 2); // btype = 1 (fixed Huffman)
        for byte in [0_u8, 12, 34, 56] {
            push_bits_msb(&mut bit_stream, 0b0011_0000 + u32::from(byte), 8);
        }
        push_bits_msb(&mut bit_stream, 1, 7); // sym 257 => len 3
        push_bits_msb(&mut bit_stream, 2, 5); // dist 2 => dist 3
        push_bits_msb(&mut bit_stream, 0, 7); // sym 256 => EOB

        let mut zlib = vec![0x78, 0x01];
        for chunk in bit_stream.chunks(8) {
            let mut b = 0_u8;
            for (i, &bit) in chunk.iter().enumerate() {
                b |= bit << i;
            }
            zlib.push(b);
        }
        let expected_raw = [0_u8, 12, 34, 56, 12, 34, 56];
        zlib.extend_from_slice(&adler32(&expected_raw).to_be_bytes());

        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&2_u32.to_be_bytes());
        ihdr.extend_from_slice(&1_u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        let mut png = Vec::from(PNG_SIG);
        push_chunk(&mut png, b"IHDR", &ihdr);
        push_chunk(&mut png, b"IDAT", &zlib);
        push_chunk(&mut png, b"IEND", &[]);

        let dec = decode_png_rgba8(&png).expect("Fixed-Huffman PNG should decode");
        assert_eq!(dec.rgba8, vec![12, 34, 56, 255, 12, 34, 56, 255]);

        assert!(decode_png_rgba8(b"not a png").is_err());
        assert!(decode_png_rgba8(&png[..16]).is_err());
    }
}
