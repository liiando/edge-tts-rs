//! DRM token generation for Microsoft Edge TTS authentication.

use chrono::{DateTime, TimeZone, Utc};

/// SHA-256 initial hash values (FIPS 180-4).
const SHA256_INITIAL: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

/// Working registers after the first 14 rounds over the service's fixed 57-byte prefix.
const SHA256_STATE: [u32; 8] = [
    0xFD9FBEA2, 0x91243CE7, 0xBE499094, 0xCD2E1E0A, 0x6C006F49, 0x04BD35AB, 0xFEB0A7E4, 0x03EFFBD2,
];

/// Prefix-only parts of message-schedule words 16..29.
const SHA256_SCHEDULE: [u32; 14] = [
    0xEB24BF9E, 0x1558691F, 0xA0C09EE2, 0x0CFA32BA, 0xA622EE8C, 0xD1DD9491, 0xC0DD25A0, 0xF9E70AB6,
    0xF0C31573, 0x7058CE4E, 0x7188E606, 0x00E5AB33, 0xD258B060, 0x72766564,
];

/// SHA-256 round constants (FIPS 180-4).
const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3, 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];

/// Round 14 constant with the prefix's last byte (the top byte of word 14) folded in.
const SHA256_ADJUSTMENT: u32 = 0xC9DC06A7;
/// sigma0 / sigma1 of that prefix byte, XORed in where word 14 meets a small sigma.
const SHA256_SIGMA0_ADJUSTMENT: u32 = 0x059C0B80;
const SHA256_SIGMA1_ADJUSTMENT: u32 = 0x000B92C0;

const CS64_COEFFICIENTS: [(u32, u32); 14] = [
    (0xE82E91B1, 0xB32700D0),
    (0x44DE24C6, 0x88DFF51D),
    (0x3BEB65BC, 0x29514038),
    (0xC89C5933, 0xB328CBDF),
    (0x8D1EC211, 0xD405B4D5),
    (0x314EF5D5, 0x26E55F3C),
    (0xE92491B1, 0x607801C6),
    (0xF0A5B9D5, 0x82B2C1FB),
    (0x06064CF3, 0xFA51A3DA),
    (0xDA79FA60, 0x207CC464),
    (0xBED20560, 0x3C3C74F3),
    (0x89F9C9F8, 0xD2D14B61),
    (0x35FA38C6, 0xBB04A2C1),
    (0xCB49E3C6, 0xC5626167),
];
const CS64_FACTOR: u32 = 0x82893933;
const CS64_ADJUSTMENT0: u32 = 0xF2F90B1A;
const CS64_ADJUSTMENT1: u32 = 0x0B1A0000;

fn rol16(value: u32) -> u32 {
    value.rotate_left(16)
}

fn small_sigma0(value: u32) -> u32 {
    value.rotate_right(7) ^ value.rotate_right(18) ^ (value >> 3)
}

fn small_sigma1(value: u32) -> u32 {
    value.rotate_right(17) ^ value.rotate_right(19) ^ (value >> 10)
}

fn expand_schedule(words: &mut [u32; 64], first_word: usize) {
    for index in first_word..64 {
        words[index] = small_sigma1(words[index - 2])
            .wrapping_add(words[index - 7])
            .wrapping_add(small_sigma0(words[index - 15]))
            .wrapping_add(words[index - 16]);
    }
}

/// SHA-256 rounds `first_round..64`, resuming from `state` (inside the first block when > 0).
fn sha256_rounds(state: [u32; 8], words: &[u32; 64], first_round: usize, constants: &[u32; 64]) -> [u32; 8] {
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
    for index in first_round..64 {
        let t1 = h
            .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
            .wrapping_add(g ^ (e & (f ^ g)))
            .wrapping_add(constants[index])
            .wrapping_add(words[index]);
        let t2 = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22))
            .wrapping_add((a & b) | (c & (a | b)));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    [a, b, c, d, e, f, g, h]
}

/// State after the first block: the 57-byte prefix (precomputed above) and `data[..7]`.
fn sha256_init(data: &[u8]) -> [u32; 8] {
    let mut words = [0u32; 64];
    words[14] = u32::from_be_bytes([0, data[0], data[1], data[2]]);
    words[15] = u32::from_be_bytes([data[3], data[4], data[5], data[6]]);
    for index in 16..30 {
        let mut value = SHA256_SCHEDULE[index - 16];
        let sigma1 = small_sigma1(words[index - 2]);
        value = value.wrapping_add(if index == 16 { sigma1 ^ SHA256_SIGMA1_ADJUSTMENT } else { sigma1 });
        if index >= 21 {
            value = value.wrapping_add(words[index - 7]);
        }
        if index == 29 {
            value = value.wrapping_add(small_sigma0(words[index - 15]) ^ SHA256_SIGMA0_ADJUSTMENT);
        }
        words[index] = value;
    }
    words[30] = small_sigma1(words[28])
        .wrapping_add(words[23])
        .wrapping_add(small_sigma0(words[15]))
        .wrapping_add(words[14])
        .wrapping_add(SHA256_ADJUSTMENT.wrapping_sub(SHA256_ROUND_CONSTANTS[14]));
    expand_schedule(&mut words, 31);
    let mut constants = SHA256_ROUND_CONSTANTS;
    constants[14] = SHA256_ADJUSTMENT;
    let working = sha256_rounds(SHA256_STATE, &words, 14, &constants);
    std::array::from_fn(|i| SHA256_INITIAL[i].wrapping_add(working[i]))
}

/// SHA-256 of the service's fixed 57-byte prefix followed by `data` (at least 7 bytes).
fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let length = 57 + data.len();
    let mut padded = data[7..].to_vec();
    padded.push(0x80);
    padded.resize(padded.len() + (55 - length as i64).rem_euclid(64) as usize, 0);
    padded.extend_from_slice(&((length as u64) * 8).to_be_bytes());
    let mut state = sha256_init(&data[..7]);
    for block in padded.chunks(64) {
        let mut words = [0u32; 64];
        for (word, bytes) in words.iter_mut().zip(block.chunks(4)) {
            *word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        expand_schedule(&mut words, 16);
        let working = sha256_rounds(state, &words, 0, &SHA256_ROUND_CONSTANTS);
        state = std::array::from_fn(|i| state[i].wrapping_add(working[i]));
    }
    let mut digest = [0u8; 32];
    for (bytes, word) in digest.chunks_mut(4).zip(state) {
        bytes.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// The two CS64 words keyed by the SHA-256 digest.
fn cs64_digest(data: &[u8], digest: &[u8; 32]) -> (u32, u32) {
    let key_a = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) | 1;
    let key_b = u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]) | 1;
    let factors = [key_a.wrapping_mul(CS64_FACTOR), key_b.wrapping_mul(CS64_FACTOR)];
    let mut terms: Vec<u32> = CS64_COEFFICIENTS
        .iter()
        .enumerate()
        .map(|(index, (coefficient, _))| coefficient.wrapping_mul(factors[index % 2]))
        .collect();
    let mut offsets: Vec<u32> = CS64_COEFFICIENTS.iter().map(|(_, offset)| *offset).collect();

    // Little-endian words of 0x00 + data, zero-padded to an even word count.
    let mut bytes = vec![0u8];
    bytes.extend_from_slice(data);
    bytes.resize(bytes.len().div_ceil(8) * 8, 0);
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let even = index % 2 == 0;
        let mut term = word.wrapping_mul(if even { key_a } else { key_b });
        let mut offset = rol16(word).wrapping_mul(if even { 0xEF0569FB } else { 0xC31713DB });
        if index == 0 {
            term = term.wrapping_add(CS64_ADJUSTMENT0.wrapping_mul(factors[0]));
            offset = offset.wrapping_add(CS64_ADJUSTMENT1);
        }
        terms.push(term);
        offsets.push(offset);
    }

    let (mut swap_acc, mut swap_last, mut state, mut acc) = (0u32, 0u32, 0u32, 0u32);
    for index in (0..terms.len()).step_by(2) {
        let (x, y) = (terms[index], terms[index + 1]);
        let q = rol16(offsets[index].wrapping_add(x)).wrapping_mul(0x76F50F34);
        swap_acc = swap_acc.wrapping_add(rol16(q).wrapping_mul(0xC3EFEA97).wrapping_add(q));
        let q = rol16(offsets[index + 1].wrapping_add(y)).wrapping_mul(0x1881CE3C);
        swap_last = rol16(q).wrapping_mul(0x35BD1EC9).wrapping_add(q);
        swap_acc = swap_acc.wrapping_add(swap_last);

        state = state.wrapping_mul(key_a).wrapping_add(x);
        state = rol16(state).wrapping_mul(0xCF98B111);
        state = rol16(state).wrapping_mul(0x87085B9F);
        state = rol16(state).wrapping_mul(0x12CEB96D);
        state = rol16(state).wrapping_mul(0x257E1D83);
        acc = acc.wrapping_add(state);
        let mut t = state.wrapping_mul(key_b).wrapping_add(y);
        t = rol16(t).wrapping_mul(0xA27416F5);
        t = rol16(t).wrapping_mul(0xD38396FF);
        state = rol16(rol16(t).wrapping_mul(0x7C932B89)).wrapping_mul(0xBFA49F69);
        acc = acc.wrapping_add(state);
    }
    (swap_last ^ state, swap_acc ^ acc)
}

/// DRM token generator for Microsoft Edge TTS authentication.
pub struct DRM;

impl DRM {
    /// Generate the Sec-MS-GEC token (version 2, sent with `Sec-MS-GEC-Version: 2-…`).
    ///
    /// The input is the current time as a Windows FILETIME (100-nanosecond
    /// intervals since 1601-01-01), floored to the nearest 5-minute boundary,
    /// followed by the trusted client token. The token is
    /// `<CS64, 16 hex>-<SHA-256, 64 hex>` in UPPERCASE, as in Python edge-tts 7.3.
    ///
    /// # Arguments
    ///
    /// * `clock_skew_seconds` - Optional clock skew adjustment in seconds
    ///
    /// # Returns
    ///
    /// A tuple containing (win_ts_100ns, token) where:
    /// - win_ts_100ns is the Windows FILETIME value used
    /// - token is the v2 Sec-MS-GEC value
    pub fn generate_sec_ms_gec(clock_skew_seconds: Option<i64>) -> (i64, String) {
        // Seconds between Windows epoch (1601-01-01) and Unix epoch (1970-01-01)
        const WIN_EPOCH: i64 = 11_644_473_600;

        let skew = clock_skew_seconds.unwrap_or(0);
        let unix_ts = Utc::now().timestamp() + skew;

        // Convert to Windows FILETIME seconds, then floor to 5-minute boundary
        let win_ts_s = unix_ts + WIN_EPOCH;
        let win_ts_s = win_ts_s - (win_ts_s % 300);

        // Convert to 100-nanosecond intervals (Windows FILETIME unit)
        let win_ts_100ns = win_ts_s * 10_000_000;

        (win_ts_100ns, Self::sec_ms_gec_for(win_ts_100ns))
    }

    /// The v2 Sec-MS-GEC token for a FILETIME already floored to 5 minutes.
    pub fn sec_ms_gec_for(win_ts_100ns: i64) -> String {
        let data = format!("{}{}", win_ts_100ns, crate::constants::TRUSTED_CLIENT_TOKEN);
        let digest = sha256_digest(data.as_bytes());
        let (low, high) = cs64_digest(data.as_bytes(), &digest);
        let mut cs64 = low.to_le_bytes().to_vec();
        cs64.extend_from_slice(&high.to_le_bytes());
        format!("{}-{}", hex::encode_upper(cs64), hex::encode_upper(digest))
    }

    /// Generate a random MUID (Machine User ID).
    ///
    /// This is used for tracking purposes in the request headers.
    ///
    /// # Returns
    ///
    /// A random UUID string without hyphens
    pub fn generate_muid() -> String {
        uuid::Uuid::new_v4()
            .to_string()
            .replace("-", "")
    }

    /// Generate a date string for the X-Ms-Date header.
    ///
    /// # Arguments
    ///
    /// * `clock_skew_seconds` - Optional clock skew adjustment in seconds
    ///
    /// # Returns
    ///
    /// A date string in RFC 1123 format (e.g., "Mon, 01 Jan 2024 12:00:00 GMT")
    pub fn generate_date(clock_skew_seconds: Option<i64>) -> String {
        let skew = clock_skew_seconds.unwrap_or(0);
        let timestamp = Utc::now().timestamp() + skew;

        // RFC 1123 format
        let dt: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(Utc::now);
        dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sec_ms_gec() {
        let (win_ts_100ns, token) = DRM::generate_sec_ms_gec(None);

        // Token is <16 hex>-<64 hex>, UPPERCASE
        let (cs64, sha256) = token.split_once('-').expect("v2 token has a dash");
        assert_eq!((cs64.len(), sha256.len()), (16, 64));
        assert!(token.chars().all(|c| c == '-' || c.is_ascii_uppercase() || c.is_ascii_digit()));

        // Timestamp should be divisible by 300 * 10_000_000 (5-minute boundary)
        assert_eq!(win_ts_100ns % (300 * 10_000_000), 0);

        // Timestamp must be positive and after 2020-01-01 in Windows FILETIME
        assert!(win_ts_100ns > 132_225_408_000_000_000);
    }

    #[test]
    fn test_sec_ms_gec_matches_python_edge_tts() {
        // generate_sec_ms_gec_v2() from Python edge-tts 7.3.1 at the same instants.
        let vectors = [
            (116444736000000000, "52BC7A17B50DFEC9-28C4394775C8751CD2F6859C030C06DB1489ACAB3B3A7651831FC26895177ED6"),
            (133444734000000000, "2CD23A2CA0B2595A-F3C846C370E05F9CC83ACC5F995C5F5D87A4D0F14CE8936A0F902E64099E38A2"),
            (134371635000000000, "B35BC21E4A2A9BB9-8FA5A16DBE2003C25CB2BFFCCAF60D72280C1DD38E12B8CA541FC3B667B8E49B"),
            (134371638000000000, "2456B152DB55870A-E5BF1224BD4B3704B668D38B81575FE51A1399C02A032DDB56B3AF78AC8C02EE"),
            (136444737000000000, "498DEBEC091E9FBB-48F566F30D363EA955F305452BB7D7029DA9E2E43FF809AEEF3ACB5964B2A49B"),
            (157469184000000000, "DD230DB44FEE2FD7-DA0D4FA797866E36C992FABB81DD2F9DB37EB74D675D63228277003DCD3F9F8C"),
        ];
        for (win_ts_100ns, expected) in vectors {
            assert_eq!(DRM::sec_ms_gec_for(win_ts_100ns), expected, "FILETIME {}", win_ts_100ns);
        }
    }

    #[test]
    fn test_generate_muid() {
        let muid = DRM::generate_muid();

        // MUID should be 32 characters (UUID without hyphens)
        assert_eq!(muid.len(), 32);

        // MUID should be lowercase hex
        assert!(muid.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_date() {
        let date = DRM::generate_date(None);

        // Date should end with GMT
        assert!(date.ends_with("GMT"));
    }
}
