// SPDX-License-Identifier: MPL-2.0

//! Latin → Thai character mapping tables.
//!
//! This module defines keyboard layout mappings. The Kedmanee layout
//! (TIS 820-2538) maps physical keys on a US QWERTY keyboard to Thai
//! characters. Both unshifted and shifted layers are covered.

/// Map a character (as typed on a US QWERTY keyboard) to its Kedmanee
/// Thai equivalent. Returns `None` for unmapped keys.
///
/// The input preserves case: lowercase = unshifted layer, uppercase =
/// shifted layer. Digits and symbols are also mapped.
pub fn kedmanee_map(ch: char) -> Option<char> {
    let mapped = match ch {
        // ── Number row (unshifted) ──────────────────────────────────
        '`' => '_',
        '1' => 'ๅ',
        '2' => '/',
        '3' => '-',
        '4' => 'ภ',
        '5' => 'ถ',
        '6' => 'ุ',
        '7' => 'ึ',
        '8' => 'ค',
        '9' => 'ต',
        '0' => 'จ',
        '-' => 'ข',
        '=' => 'ช',

        // ── Number row (shifted) ────────────────────────────────────
        '~' => '%',
        '!' => '+',
        '@' => '๑',
        '#' => '๒',
        '$' => '๓',
        '%' => '๔',
        '^' => 'ู',
        '&' => '฿',
        '*' => '๕',
        '(' => '๖',
        ')' => '๗',
        '_' => '๘',
        '+' => '๙',

        // ── Top letter row (unshifted: qwertyuiop[]) ───────────────
        'q' => 'ๆ',
        'w' => 'ไ',
        'e' => 'ำ',
        'r' => 'พ',
        't' => 'ะ',
        'y' => 'ั',
        'u' => 'ี',
        'i' => 'ร',
        'o' => 'น',
        'p' => 'ย',
        '[' => 'บ',
        ']' => 'ล',
        '\\' => 'ฃ',

        // ── Top letter row (shifted: QWERTYUIOP{}) ─────────────────
        'Q' => '๐',
        'W' => '"',
        'E' => 'ฎ',
        'R' => 'ฑ',
        'T' => 'ธ',
        'Y' => 'ํ',
        'U' => '๊',
        'I' => 'ณ',
        'O' => 'ฯ',
        'P' => 'ญ',
        '{' => 'ฐ',
        '}' => ',',
        '|' => 'ฅ',

        // ── Home row (unshifted: asdfghjkl;') ──────────────────────
        'a' => 'ฟ',
        's' => 'ห',
        'd' => 'ก',
        'f' => 'ด',
        'g' => 'เ',
        'h' => '้',
        'j' => '่',
        'k' => 'า',
        'l' => 'ส',
        ';' => 'ว',
        '\'' => 'ง',

        // ── Home row (shifted: ASDFGHJKL:") ────────────────────────
        'A' => 'ฤ',
        'S' => 'ฆ',
        'D' => 'ฏ',
        'F' => 'โ',
        'G' => 'ฌ',
        'H' => '็',
        'J' => '๋',
        'K' => 'ษ',
        'L' => 'ศ',
        ':' => 'ซ',
        '"' => '.',

        // ── Bottom row (unshifted: zxcvbnm,./) ─────────────────────
        'z' => 'ผ',
        'x' => 'ป',
        'c' => 'แ',
        'v' => 'อ',
        'b' => 'ิ',
        'n' => 'ื',
        'm' => 'ท',
        ',' => 'ม',
        '.' => 'ใ',
        '/' => 'ฝ',

        // ── Bottom row (shifted: ZXCVBNM<>?) ───────────────────────
        'Z' => '(',
        'X' => ')',
        'C' => 'ฉ',
        'V' => 'ฮ',
        'B' => 'ฺ',
        'N' => '์',
        'M' => '?',
        '<' => 'ฒ',
        '>' => 'ฬ',
        '?' => 'ฦ',

        _ => return None,
    };
    Some(mapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kedmanee_unshifted_letters() {
        // Home row
        assert_eq!(kedmanee_map('a'), Some('ฟ'));
        assert_eq!(kedmanee_map('s'), Some('ห'));
        assert_eq!(kedmanee_map('d'), Some('ก'));
        assert_eq!(kedmanee_map('f'), Some('ด'));
        assert_eq!(kedmanee_map('g'), Some('เ'));
        assert_eq!(kedmanee_map('h'), Some('้'));
        assert_eq!(kedmanee_map('j'), Some('่'));
        assert_eq!(kedmanee_map('k'), Some('า'));
        assert_eq!(kedmanee_map('l'), Some('ส'));

        // Top row
        assert_eq!(kedmanee_map('q'), Some('ๆ'));
        assert_eq!(kedmanee_map('w'), Some('ไ'));
        assert_eq!(kedmanee_map('e'), Some('ำ'));
        assert_eq!(kedmanee_map('r'), Some('พ'));
        assert_eq!(kedmanee_map('t'), Some('ะ'));
        assert_eq!(kedmanee_map('y'), Some('ั'));
        assert_eq!(kedmanee_map('u'), Some('ี'));
        assert_eq!(kedmanee_map('i'), Some('ร'));
        assert_eq!(kedmanee_map('o'), Some('น'));
        assert_eq!(kedmanee_map('p'), Some('ย'));

        // Bottom row
        assert_eq!(kedmanee_map('z'), Some('ผ'));
        assert_eq!(kedmanee_map('x'), Some('ป'));
        assert_eq!(kedmanee_map('c'), Some('แ'));
        assert_eq!(kedmanee_map('v'), Some('อ'));
        assert_eq!(kedmanee_map('b'), Some('ิ'));
        assert_eq!(kedmanee_map('n'), Some('ื'));
        assert_eq!(kedmanee_map('m'), Some('ท'));
    }

    #[test]
    fn test_kedmanee_shifted_letters() {
        assert_eq!(kedmanee_map('A'), Some('ฤ'));
        assert_eq!(kedmanee_map('S'), Some('ฆ'));
        assert_eq!(kedmanee_map('D'), Some('ฏ'));
        assert_eq!(kedmanee_map('F'), Some('โ'));
        assert_eq!(kedmanee_map('G'), Some('ฌ'));
        assert_eq!(kedmanee_map('H'), Some('็'));
        assert_eq!(kedmanee_map('J'), Some('๋'));
        assert_eq!(kedmanee_map('K'), Some('ษ'));
        assert_eq!(kedmanee_map('L'), Some('ศ'));

        assert_eq!(kedmanee_map('Q'), Some('๐'));
        assert_eq!(kedmanee_map('W'), Some('"'));
        assert_eq!(kedmanee_map('E'), Some('ฎ'));
        assert_eq!(kedmanee_map('R'), Some('ฑ'));
        assert_eq!(kedmanee_map('T'), Some('ธ'));
        assert_eq!(kedmanee_map('U'), Some('๊'));
        assert_eq!(kedmanee_map('I'), Some('ณ'));
        assert_eq!(kedmanee_map('O'), Some('ฯ'));
        assert_eq!(kedmanee_map('P'), Some('ญ'));

        assert_eq!(kedmanee_map('C'), Some('ฉ'));
        assert_eq!(kedmanee_map('V'), Some('ฮ'));
        assert_eq!(kedmanee_map('N'), Some('์'));
        assert_eq!(kedmanee_map('Z'), Some('('));
        assert_eq!(kedmanee_map('X'), Some(')'));
    }

    #[test]
    fn test_kedmanee_digits() {
        assert_eq!(kedmanee_map('1'), Some('ๅ'));
        assert_eq!(kedmanee_map('2'), Some('/'));
        assert_eq!(kedmanee_map('3'), Some('-'));
        assert_eq!(kedmanee_map('4'), Some('ภ'));
        assert_eq!(kedmanee_map('5'), Some('ถ'));
        assert_eq!(kedmanee_map('6'), Some('ุ'));
        assert_eq!(kedmanee_map('7'), Some('ึ'));
        assert_eq!(kedmanee_map('8'), Some('ค'));
        assert_eq!(kedmanee_map('9'), Some('ต'));
        assert_eq!(kedmanee_map('0'), Some('จ'));
    }

    #[test]
    fn test_kedmanee_shifted_digits() {
        assert_eq!(kedmanee_map('!'), Some('+'));
        assert_eq!(kedmanee_map('@'), Some('๑'));
        assert_eq!(kedmanee_map('#'), Some('๒'));
        assert_eq!(kedmanee_map('$'), Some('๓'));
        assert_eq!(kedmanee_map('%'), Some('๔'));
        assert_eq!(kedmanee_map('^'), Some('ู'));
        assert_eq!(kedmanee_map('&'), Some('฿'));
        assert_eq!(kedmanee_map('*'), Some('๕'));
        assert_eq!(kedmanee_map('('), Some('๖'));
        assert_eq!(kedmanee_map(')'), Some('๗'));
        assert_eq!(kedmanee_map('_'), Some('๘'));
        assert_eq!(kedmanee_map('+'), Some('๙'));
    }

    #[test]
    fn test_kedmanee_symbols() {
        assert_eq!(kedmanee_map('`'), Some('_'));
        assert_eq!(kedmanee_map('-'), Some('ข'));
        assert_eq!(kedmanee_map('='), Some('ช'));
        assert_eq!(kedmanee_map('['), Some('บ'));
        assert_eq!(kedmanee_map(']'), Some('ล'));
        assert_eq!(kedmanee_map('\\'), Some('ฃ'));
        assert_eq!(kedmanee_map(';'), Some('ว'));
        assert_eq!(kedmanee_map('\''), Some('ง'));
        assert_eq!(kedmanee_map(','), Some('ม'));
        assert_eq!(kedmanee_map('.'), Some('ใ'));
        assert_eq!(kedmanee_map('/'), Some('ฝ'));
    }

    #[test]
    fn test_kedmanee_shifted_symbols() {
        assert_eq!(kedmanee_map('~'), Some('%'));
        assert_eq!(kedmanee_map('{'), Some('ฐ'));
        assert_eq!(kedmanee_map('}'), Some(','));
        assert_eq!(kedmanee_map('|'), Some('ฅ'));
        assert_eq!(kedmanee_map(':'), Some('ซ'));
        assert_eq!(kedmanee_map('"'), Some('.'));
        assert_eq!(kedmanee_map('<'), Some('ฒ'));
        assert_eq!(kedmanee_map('>'), Some('ฬ'));
        assert_eq!(kedmanee_map('?'), Some('ฦ'));
    }

    #[test]
    fn test_kedmanee_unmapped() {
        assert_eq!(kedmanee_map(' '), None);
        assert_eq!(kedmanee_map('\t'), None);
        assert_eq!(kedmanee_map('\n'), None);
        assert_eq!(kedmanee_map('ก'), None); // Thai char
    }

    #[test]
    fn test_kedmanee_full_coverage() {
        // Every printable ASCII key on a US QWERTY keyboard should be mapped
        let all_mapped: &str = concat!(
            "`1234567890-=",
            "qwertyuiop[]\\",
            "asdfghjkl;'",
            "zxcvbnm,./",
            "~!@#$%^&*()_+",
            "QWERTYUIOP{}|",
            "ASDFGHJKL:\"",
            "ZXCVBNM<>?",
        );
        for ch in all_mapped.chars() {
            assert!(
                kedmanee_map(ch).is_some(),
                "Expected mapping for {:?} but got None",
                ch
            );
        }
    }
}
