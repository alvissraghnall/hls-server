pub fn encode_uri(input: impl AsRef<str>) -> String {
    let input = input.as_ref();

    let mut out = String::with_capacity(input.len());

    for b in input.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'_'
            | b'-'
            | b'.'
            | b':'
            | b'/'
            | b'\\' => {
                out.push(b as char);
            }

            _ => {
                out.push('%');
                out.push(hex((b >> 4) & 0xF));
                out.push(hex(b & 0xF));
            }
        }
    }

    out
}

fn hex(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + (n - 10)) as char,
        _ => unreachable!(),
    }
}

pub fn decode_uri(input: impl AsRef<str>) -> String {
    let input = input.as_ref().as_bytes();

    let mut out = Vec::with_capacity(input.len());

    let mut i = 0;

    while i < input.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            let hi = from_hex(input[i + 1]);
            let lo = from_hex(input[i + 2]);

            match (hi, lo) {
                (Some(h), Some(l)) => {
                    out.push((h << 4) | l);
                    i += 3;
                    continue;
                }

                _ => {}
            }
        }

        out.push(input[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}