//! Shareable world seeds: a short code such as "K7Q2X" (or any word) picks a world, so friends
//! can play the same one.

/// Random codes only use characters that can't be mistaken for each other (no 0/O or 1/I/L).
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
/// Length of a random code.
pub const CODE_LEN: usize = 5;
/// Longest code a player can type.
pub const MAX_LEN: usize = 12;

/// Upper-cases, keeps only ASCII letters and digits, and stops at `MAX_LEN` characters.
pub fn normalize(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .take(MAX_LEN)
        .collect()
}

/// A random code, built from `bits`.
pub fn random_code(mut bits: u64) -> String {
    let base = ALPHABET.len() as u64;
    (0..CODE_LEN)
        .map(|_| {
            let c = ALPHABET[(bits % base) as usize] as char;
            bits /= base;
            c
        })
        .collect()
}

/// The world-generation seed for a normalized code. Integer-only, so every platform builds the
/// same world. Changing this changes every world and breaks codes people have already shared.
pub fn world_seed(code: &str) -> u64 {
    // FNV-1a...
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in code.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    // ...then the splitmix64 finalizer, so similar codes give unrelated worlds.
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_upper_cases_and_keeps_only_letters_and_digits() {
        assert_eq!(normalize(" k7q-2x! "), "K7Q2X");
        assert_eq!(normalize("banana"), "BANANA");
        assert_eq!(normalize("é ü 😀"), "");
    }

    #[test]
    fn normalize_stops_at_max_len() {
        assert_eq!(normalize("abcdefghijklmnop"), "ABCDEFGHIJKL");
        assert_eq!(normalize("abcdefghijklmnop").len(), MAX_LEN);
    }

    #[test]
    fn random_code_is_short_and_unambiguous() {
        assert_eq!(random_code(0), "22222");
        assert_eq!(random_code(1), "32222");
        assert_eq!(random_code(31), "23222");
        assert_eq!(random_code(123_456_789), "425QB");
        for bits in [0, 7, 1 << 40, u64::MAX] {
            let code = random_code(bits);
            assert_eq!(code.len(), CODE_LEN);
            assert!(code.bytes().all(|b| ALPHABET.contains(&b)), "{code}");
            assert_eq!(normalize(&code), code, "random codes are already normalized");
        }
    }

    #[test]
    fn world_seed_is_pinned() {
        // Golden values computed independently. If these change, every shared code breaks.
        assert_eq!(world_seed("BANANA"), 0x98FF_0611_5810_37FD);
        assert_eq!(world_seed("K7Q2X"), 0x4FD0_000E_75F4_D364);
    }

    #[test]
    fn similar_codes_give_different_seeds() {
        assert_ne!(world_seed("K7Q2X"), world_seed("K7Q2Y"));
        assert_ne!(world_seed("A"), world_seed("B"));
    }
}
